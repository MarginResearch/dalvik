use std::{collections::HashSet, path::PathBuf};

use clap::Parser;
use dex::{jtype::TypeId, Endian};

#[derive(Debug, Parser)]
struct Args {
    /// Input dex file
    #[arg(value_name("DEX FILE"))]
    file: PathBuf,

    /// Class string, e.g. com.example.MyClass
    class: String,

    /// Method to dump with try/catch indentation
    method: String,
}

impl Args {
    fn normalize(mut self) -> Self {
        // turn class string like "com.example.MyClass" into typdef string, e.g.
        // "Lcom/example/MyClass;"
        self.class = format!("L{};", self.class);
        self.class = self.class.replace('.', "/");

        self
    }
}

fn main() {
    let args = Args::parse().normalize();

    let dex_bytes = std::fs::read(args.file).unwrap();
    let dex = dex::DexReader::from_vec(&dex_bytes).unwrap();
    assert_eq!(
        dex.get_endian(),
        Endian::Little,
        "Big Endian dex file encountered. TODO: idk if bytecodes are swapped in big endian modes or not"
    );

    let class = dex.find_class_by_name(&args.class).unwrap().unwrap();

    let method = class
        .methods()
        .chain(class.direct_methods())
        .chain(class.virtual_methods())
        .find(|m| m.name().as_ref() == args.method)
        .unwrap();

    dump_graphviz(method, &dex, &dex_bytes);
}

fn dump_graphviz<T: AsRef<[u8]>>(method: &dex::method::Method, dex: &dex::Dex<T>, bytes: &[u8]) {
    let Some(code) = method.code() else {
        return;
    };
    println!("digraph {{");
    println!("    nojustify=true");
    // Fonts are handled poorly in graphviz, and font-lookup is very
    // system-dependent. This probably won't work on your machine, but even
    // silently using the fallback font beats Times Roman.
    println!("    node [shape=box margin=\"0.8,0.1\" fontname=\"Agave Nerd Font\"]");

    use dalvik::PrettyPrint;
    let mylookup = MyLookup { dex, bytes };

    let bytecode = code.insns().as_slice();

    let mut catch_addrs = code
        .tries()
        .try_catch_blocks()
        .iter()
        .flat_map(|tc| tc.catch_handlers().iter().map(|c| c.addr() as usize).chain([tc.start_addr() as usize]))
        .collect::<Vec<_>>();
    catch_addrs.sort_unstable();

    let basic_blocks = dalvik::blocks::basic_blocks(bytecode, &catch_addrs);

    let mut disassembly = String::new();
    for (id, bb) in &basic_blocks {
        disassembly.push_str(&format!("    {id} [label=\""));
        for inst in &bb.instructions {
            disassembly.push_str(&mylookup.print(&inst).replace('"', "\\\""));
            disassembly.push_str("\\l");
        }
        disassembly.push_str("\"]");
        println!("{disassembly}");
        disassembly.clear();
    }

    println!();

    let mut catch_edges = HashSet::new();

    // connect the catch nodes
    for tc in code.tries().try_catch_blocks() {
        let first_addr = tc.start_addr() as usize;
        let last_addr = first_addr + tc.insn_count() as usize;
        for catch in tc.catch_handlers() {
            let c = catch.addr();
            let exception = match catch.exception() {
                dex::code::ExceptionType::BaseException => "BaseException".into(),
                dex::code::ExceptionType::Ty(t) => t.to_string(),
            };
            // create a node to put the exception type in, because labelling edges can get confusing
            println!("    catch{c} [label=\"catch {exception}\"]");

            // draw all edges to this catch node
            for (addr, _block) in &basic_blocks {
                if *addr >= first_addr && *addr < last_addr {
                    println!("    {addr} -> catch{c} [style=dashed]");
                }
            }

            // draw the edge from the catch node to the disassembly
            // (filtered though a HashSet so we only ever draw one edge)
            catch_edges.insert(c);
        }
    }

    // connect the catch nodes with the associated disassembly
    for c in catch_edges {
        println!("    catch{c} -> {c} [penwidth=2]");
    }

    // connect the normal block flow
    for (id, bb) in basic_blocks {
        use dalvik::blocks::NextBranch;
        match bb.next {
            NextBranch::Cond { t, f } => {
                println!("    {id} -> {t} [color=green weight=10 headport=n]");
                println!("    {id} -> {f} [color=red weight=5 headport=n]");
            }
            NextBranch::Goto(n) => println!("    {id} -> {n} [weight=15 penwidth=2 headport=n]"),
            NextBranch::None => continue,
        }
    }

    println!("}}");
}

struct MyLookup<'a, T> {
    dex: &'a dex::Dex<T>,
    bytes: &'a [u8],
}

impl<'a, T: AsRef<[u8]>> dalvik::PrettyPrint for MyLookup<'a, T> {
    fn method(&self, index: u16) -> (String, String, String, String) {
        let method = self.dex.get_method_item(index.into()).unwrap();
        let class = self.dex.get_type((method.class_idx()).into()).unwrap();
        let name = self.dex.get_string(method.name_idx()).unwrap();

        let proto = self.dex.get_proto_item(method.proto_idx().into()).unwrap();
        let params = param_type_ids(self.bytes, proto.params_off());
        let mut paramstr = String::new();
        for p in params {
            let param = self.dex.get_type(p).unwrap();
            paramstr.push_str(&param.to_string());
        }

        let ret = self.dex.get_type(proto.return_type()).unwrap();

        (class.to_string(), name.to_string(), paramstr, ret.to_string())
    }

    fn field(&self, index: u16) -> (String, String, String) {
        let field = self.dex.get_field_item(index.into()).unwrap();
        let class = self.dex.get_type((*field.class_idx()).into()).unwrap();
        let ty = self.dex.get_type((*field.type_idx()).into()).unwrap();
        let name = self.dex.get_string(*field.name_idx()).unwrap();

        (class.to_string(), name.to_string(), ty.to_string())
    }

    fn string(&self, index: u32) -> String {
        self.dex.get_string(index.into()).unwrap().to_string()
    }

    fn type_name(&self, index: u16) -> String {
        let ty = self.dex.get_type(index.into()).unwrap();
        ty.to_string()
    }
}

fn param_type_ids(buf: &[u8], offset: u32) -> Vec<TypeId> {
    if offset == 0 {
        return Vec::new();
    }

    let (_, buf) = buf.split_at(offset as usize);

    // read the param length
    let (len, mut buf) = buf.split_at(4);
    let len = u32::from_le_bytes(len.try_into().unwrap());

    let mut types = Vec::new();
    for _ in 0..len {
        // read the param length
        let (ty, rest) = buf.split_at(2);
        let ty = u16::from_le_bytes(ty.try_into().unwrap());
        types.push(ty.into());
        buf = rest;
    }

    types
}
