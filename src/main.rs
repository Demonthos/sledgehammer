#![allow(non_camel_case_types)]

// mod attrs;
mod attribute;
mod builder;
mod element;
mod event;
mod value;

pub use attribute::*;
pub use builder::*;
pub use element::*;

use dioxus_interpreter_js::Interpreter;

use easybench_wasm::bench as wasm_bench;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{console, Document, HtmlHeadElement, Node, Performance};

const CUSTOMIZATIONS: usize = 0;
const ELEMENTS: usize = 1;
const ID: Option<u64> = Some(1);
const NO_ID: Option<u64> = None;

static mut PTR: u32 = 0;
static mut PTR_PTR: *const u32 = unsafe { &PTR } as *const u32;
static mut LEN_PTR: u32 = 0;
static mut LEN_PTR_PTR: *const u32 = unsafe { &LEN_PTR } as *const u32;

#[wasm_bindgen(module = "/interpreter.js")]
extern "C" {
    fn interperter_init(mem: JsValue, ptr: u32, size: u32);

    #[wasm_bindgen(js_name = "work")]
    fn work_inner();

    fn set_node(id: u64, node: Node);

    fn bench();

    fn bench_template();

    fn prep();
}

fn work(data: &[u8]) {
    let ptr = data.as_ptr();
    let len = data.len();
    unsafe {
        PTR = ptr as u32;
        LEN_PTR = len as u32
    };
    work_inner();

    let _ = data;
}

fn bench_hand(perf: &Performance) {
    let res = wasm_bench(|| {
        prep();
        // const MSG: &[u8; ELEMENTS * 32] = &[
        //     8, 1, 16, 14, 1, 53, 255, 8, 2, 36, 14, 2, 19, 4, 116, 101, 115, 116, 2, 1, 15, 1, 53,
        //     8, 0, 59, 5, 2, 1, 0, 0, 0, 8, 1, 16, 14, 1, 53, 255, 8, 2, 36, 14, 2, 19, 4, 116, 101,
        //     115, 116, 2, 1, 15, 1, 53, 8, 0, 59, 5, 2, 1, 0, 0, 0, 8, 1, 16, 14, 1, 53, 255, 8, 2,
        //     36, 14, 2, 19, 4, 116, 101, 115, 116, 2, 1, 15, 1, 53, 8, 0, 59, 5, 2, 1, 0, 0, 0, 8,
        //     1, 16, 14, 1, 53, 255, 8, 2, 36, 14, 2, 19, 4, 116, 101, 115, 116, 2, 1, 15, 1, 53, 8,
        //     0, 59, 5, 2, 1, 0, 0, 0, 8, 1, 16, 14, 1, 53, 255, 8, 2, 36, 14, 2, 19, 4, 116, 101,
        //     115, 116, 2, 1, 15, 1, 53, 8, 0, 59, 5, 2, 1, 0, 0, 0, 8, 1, 16, 14, 1, 53, 255, 8, 2,
        //     36, 14, 2, 19, 4, 116, 101, 115, 116, 2, 1, 15, 1, 53, 8, 0, 59, 5, 2, 1, 0, 0, 0, 8,
        //     1, 16, 14, 1, 53, 255, 8, 2, 36, 14, 2, 19, 4, 116, 101, 115, 116, 2, 1, 15, 1, 53, 8,
        //     0, 59, 5, 2, 1, 0, 0, 0, 8, 1, 16, 14, 1, 53, 255, 8, 2, 36, 14, 2, 19, 4, 116, 101,
        //     115, 116, 2, 1, 15, 1, 53, 8, 0, 59, 5, 2, 1, 0, 0, 0, 8, 1, 16, 14, 1, 53, 255, 8, 2,
        //     36, 14, 2, 19, 4, 116, 101, 115, 116, 2, 1, 15, 1, 53, 8, 0, 59, 5, 2, 1, 0, 0, 0, 8,
        //     1, 16, 14, 1, 53, 255, 8, 2, 36, 14, 2, 19, 4, 116, 101, 115, 116, 2, 1, 15, 1, 53, 8,
        //     0, 59, 5, 2, 1, 0, 0, 0,
        // ];
        // let ptr = MSG.as_ptr();
        // let len = MSG.len();
        // unsafe {
        //     PTR = ptr as u32;
        //     LEN_PTR = len as u32
        // };
        // work_inner();
    });
    console::log_1(&format!(" msg.create_element hand\n{}", res).into());
}

fn bench_msg_element(perf: &Performance) {
    let res = wasm_bench(|| {
        prep();
        let mut msg = MsgBuilder::new();
        for _ in 0..ELEMENTS {
            msg.create_element(Element::blockquote, ID);
            msg.set_attribute(Attribute::hidden, true, ID);
            msg.create_element(Element::div, ID.map(|id| id + 1));
            msg.set_attribute(Attribute::class, &"test", ID.map(|id| id + 1));
            msg.create_element(Element::input, NO_ID);
            msg.append_children(2);
        }
        msg.build();
    });
    console::log_1(&format!(" msg.create_element\n{}", res).into());
}

fn bench_msg_element_builder(perf: &Performance) {
    let res = wasm_bench(|| {
        prep();
        const EL: ElementBuilder<
            Element,
            ((Attribute, bool),),
            (
                ElementBuilder<Element, ((Attribute, &&str),), ()>,
                ElementBuilder<Element, (), ()>,
            ),
        > = ElementBuilder::new(
            None,
            Element::blockquote,
            ((Attribute::hidden, true),),
            (
                ElementBuilder::new(None, Element::div, ((Attribute::class, &"test"),), ()),
                ElementBuilder::new(None, Element::input, (), ()),
            ),
        );
        let mut msg = MsgBuilder::new();
        for _ in 0..ELEMENTS {
            msg.create_full_element(EL);
        }
        msg.build();
    });
    console::log_1(&format!(" msg.create_element builder\n{}", res).into());
}

fn bench_msg_element_builder_prealoc(perf: &Performance) {
    const EL: ElementBuilder<
        Element,
        ((Attribute, bool),),
        (
            ElementBuilder<Element, ((Attribute, &&str),), ()>,
            ElementBuilder<Element, (), ()>,
        ),
    > = ElementBuilder::new(
        None,
        Element::blockquote,
        ((Attribute::hidden, true),),
        (
            ElementBuilder::new(None, Element::div, ((Attribute::class, &"test"),), ()),
            ElementBuilder::new(None, Element::input, (), ()),
        ),
    );
    let res = wasm_bench(|| {
        prep();
        for _ in 0..ELEMENTS {
            EL.build();
        }
    });
    console::log_1(&format!(" msg.create_element builder prealoc\n{}", res).into());
}

fn bench_msg_element_builder_clone(perf: &Performance) {
    const EL: ElementBuilder<
        Element,
        ((Attribute, bool),),
        (
            ElementBuilder<Element, ((Attribute, &&str),), ()>,
            ElementBuilder<Element, (), ()>,
        ),
    > = ElementBuilder::new(
        None,
        Element::blockquote,
        ((Attribute::hidden, true),),
        (
            ElementBuilder::new(Some(1), Element::div, ((Attribute::class, &"test"),), ()),
            ElementBuilder::new(None, Element::input, (), ()),
        ),
    );
    EL.create_template(1);
    let res = wasm_bench(|| {
        prep();
        let vec = Vec::with_capacity(5 * ELEMENTS);
        let mut msg = MsgBuilder::with(vec);
        for i in 0..ELEMENTS {
            msg.create_template_ref(1, ID);
            for _ in 0..CUSTOMIZATIONS {
                msg.set_attribute(Attribute::class, &i.to_string(), (1, 0));
            }
        }
        msg.build();
    });
    console::log_1(&format!(" msg.create_element builder clone\n{}", res).into());
}

fn bench_msg_element_builder_create_template(perf: &Performance) {
    const EL: ElementBuilder<
        Element,
        ((Attribute, bool),),
        (
            ElementBuilder<Element, ((Attribute, &&str),), ()>,
            ElementBuilder<Element, (), ()>,
        ),
    > = ElementBuilder::new(
        None,
        Element::blockquote,
        ((Attribute::hidden, true),),
        (
            ElementBuilder::new(Some(0), Element::div, ((Attribute::class, &"test"),), ()),
            ElementBuilder::new(None, Element::input, (), ()),
        ),
    );
    let res = wasm_bench(|| {
        prep();
        EL.create_template(0);
    });
    console::log_1(&format!(" msg.create_element builder create template\n{}", res).into());
}

fn bench_msg_pre_alloc(perf: &Performance) {
    const LEN: usize = 32 * ELEMENTS;
    let res = wasm_bench(|| {
        prep();
        let vec = Vec::with_capacity(LEN);
        // let vec: SmallVec<[u8; LEN]> = SmallVec::new_const();
        let mut msg = MsgBuilder::with(vec);
        for _ in 0..ELEMENTS {
            msg.create_element(Element::blockquote, ID);
            msg.set_attribute(Attribute::hidden, true, ID);
            msg.create_element(Element::div, ID.map(|id| id + 1));
            msg.set_attribute(Attribute::class, &"test", ID.map(|id| id + 1));
            msg.create_element(Element::input, NO_ID);
            msg.append_children(2);
        }
        msg.build();
    });
    console::log_1(&format!(" msg.create_element prealoc\n{}", res).into());
}

fn bench_msg_element_custom(perf: &Performance) {
    let res = wasm_bench(|| {
        prep();
        let mut msg = MsgBuilder::new();
        for _ in 0..ELEMENTS {
            msg.create_element("blockquote", ID);
            msg.set_attribute("hidden", true, ID);
            msg.create_element("div", ID.map(|id| id + 1));
            msg.set_attribute("class", &"test", ID.map(|id| id + 1));
            msg.append_children(1);
            // msg.remove_attribute("hidden", ID.unwrap());
            msg.create_element("input", NO_ID);
            // msg.insert_after(ID.unwrap() + 1, 1);
        }
        msg.build();
    });
    console::log_1(&format!(" msg.create_element custom\n{}", res).into());
}

fn bench_msg_custom_element(perf: &Performance) {
    let res = wasm_bench(|| {
        prep();
        let mut msg = MsgBuilder::new();
        for _ in 0..ELEMENTS {
            msg.create_element("blockquote", NO_ID);
            msg.create_element("div", NO_ID);
            msg.append_children(1);
        }
        msg.build();
    });
    console::log_1(&format!(" msg.create_element custom\n{}", res).into());
}

fn bench_msg_custom_element_alloc(perf: &Performance) {
    const LEN2: usize = ("blockquote".len() + "div".len() + 8) * ELEMENTS;
    let res = wasm_bench(|| {
        prep();
        let vec = Vec::with_capacity(LEN2);
        let mut msg = MsgBuilder::with(vec);
        for _ in 0..ELEMENTS {
            msg.create_element("blockquote", NO_ID);
            msg.create_element("div", NO_ID);
            msg.append_children(1);
        }
        msg.build();
    });
    console::log_1(&format!(" msg.create_element custom prealoc\n{}", res).into());
}

fn bench_msg_set_attribute(perf: &Performance) {
    let res = wasm_bench(|| {
        let mut msg = MsgBuilder::new();
        for _ in 0..ELEMENTS {
            msg.set_attribute(Attribute::alt, &"true", NO_ID);
        }
        msg.build();
    });
    console::log_1(&format!(" msg.set_attribute\n{}", res).into());
}

fn bench_msg_combined(perf: &Performance) {
    let res = wasm_bench(|| {
        let mut msg = MsgBuilder::new();
        for _ in 0..ELEMENTS {
            msg.create_element(Element::blockquote, NO_ID);
            msg.set_attribute(Attribute::alt, &"true", NO_ID);
        }
        msg.build();
    });
    console::log_1(&format!(" msg.combined\n{}", res).into());
}

fn bench_set_attribute(head: &HtmlHeadElement, perf: &Performance) {
    let res = wasm_bench(|| {
        for _ in 0..ELEMENTS {
            head.set_attribute("alt", "true").unwrap();
        }
    });
    console::log_1(&format!(" set_attribute\n{}", res).into());
}

fn bench_create_element(doc: &Document, perf: &Performance) {
    let res = wasm_bench(|| {
        for _ in 0..ELEMENTS {
            let block = doc.create_element("blockquote").unwrap();
            block.set_attribute("hidden", "true").unwrap();
            let div = doc.create_element("div").unwrap();
            div.set_attribute("class", "test").unwrap();
            let input = doc.create_element("input").unwrap();
            block.append_child(&div).unwrap();
            block.append_child(&input).unwrap();
        }
    });
    console::log_1(&format!(" create_element (web-sys)\n{}", res).into());
}

fn bench_create_element_clone(doc: &Document, perf: &Performance) {
    let block = doc.create_element("blockquote").unwrap();
    block.set_attribute("hidden", "true").unwrap();
    let div = doc.create_element("div").unwrap();
    div.set_attribute("class", "test").unwrap();
    let input = doc.create_element("input").unwrap();
    block.append_child(&div).unwrap();
    block.append_child(&input).unwrap();
    let res = wasm_bench(|| {
        for _ in 0..ELEMENTS {
            let el = block.clone_node_with_deep(true).unwrap();
            for i in 0..CUSTOMIZATIONS {
                let element: web_sys::Element = JsCast::unchecked_into(el.first_child().unwrap());
                element.set_attribute("class", &i.to_string()).unwrap();
            }
        }
    });
    console::log_1(&format!("create_element clone (web-sys)\n{}", res).into());
}

fn bench_dioxus(doc: &Document, perf: &Performance) {
    let res = wasm_bench(|| {
        let root = doc.create_element("div").unwrap();
        let interpreter = Interpreter::new(root);
        for _ in 0..ELEMENTS {
            interpreter.CreateElement("blockquote", 1);
            interpreter.SetAttribute(1, "hidden", "true".into(), None);
            interpreter.CreateElement("div", 2);
            interpreter.SetAttribute(1, "class", "test".into(), None);
            interpreter.CreateElement("input", 3);
            interpreter.AppendChildren(2);
        }
    });
    console::log_1(&format!(" dioxus\n{}", res).into());
}

pub fn main() {
    unsafe {
        interperter_init(wasm_bindgen::memory(), PTR_PTR as u32, LEN_PTR_PTR as u32);
    }

    let win = web_sys::window().unwrap();
    let doc = win.document().unwrap();
    let perf = win.performance().unwrap();

    for _ in 0..1 {
        bench_dioxus(&doc, &perf);

        // bench_hand(&perf);

        bench_msg_element(&perf);

        bench_msg_pre_alloc(&perf);

        // bench_msg_element_custom(&perf);

        // bench_msg_custom_element(&perf);

        // bench_msg_custom_element_alloc(&perf);

        bench_msg_element_builder(&perf);

        bench_create_element(&doc, &perf);

        bench_msg_element_builder_prealoc(&perf);

        bench_create_element_clone(&doc, &perf);

        bench_msg_element_builder_clone(&perf);

        bench_msg_element_builder_create_template(&perf);

        bench();

        bench_template();
    }
}
