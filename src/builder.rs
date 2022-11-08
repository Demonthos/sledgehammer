use std::{fmt::Arguments, io::Write};

use web_sys::Node;

use crate::{
    batch::{Batch, FinalizedBatch, Op},
    last_needs_memory, update_last_memory, work_last_created, ElementBuilder, IntoAttribue,
    IntoElement, JsInterpreter, MSG_METADATA_PTR, MSG_PTR_PTR, STR_LEN_PTR, STR_PTR_PTR,
};

static mut INTERPRETER_EXISTS: bool = false;

/// An id that may be either the last node or a node with an assigned id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaybeId {
    /// The last node that was created or navigated to.
    LastNode,
    /// A node that was created and stored with an id
    Node(NodeId),
}

impl MaybeId {
    #[inline(always)]
    pub(crate) const fn encoded_size(&self) -> u8 {
        match self {
            MaybeId::LastNode => 0,
            MaybeId::Node(_) => 4,
        }
    }
}

/// A node that was created and stored with an id
/// It is recommended to create and store ids with a slab allocator with an exposed slab index for example the excellent [slab](https://docs.rs/slab) crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

/// The [`MsgChannel`] handles communication with the dom. It allows you to send batched operations to the dom.
/// All of the functions that are not marked otherwise are qued and not exicuted imidately. When you want to exicute the que you have to call [`MsgChannel::flush`].
/// There should only be one msg channel per program.
pub struct MsgChannel {
    pub(crate) js_interpreter: JsInterpreter,
    batch: Batch,
}

impl Default for MsgChannel {
    fn default() -> Self {
        unsafe {
            assert!(
                !INTERPRETER_EXISTS,
                "Found another MsgChannel. Only one MsgChannel can be created"
            );
            INTERPRETER_EXISTS = true;
        }
        assert!(0x1F > Op::NoOp as u8);
        format!(
            "init: {:?}, {:?}, {:?}",
            unsafe { MSG_PTR_PTR as usize },
            unsafe { STR_PTR_PTR as usize },
            unsafe { STR_LEN_PTR as usize }
        );
        let js_interpreter = unsafe {
            JsInterpreter::new(
                wasm_bindgen::memory(),
                MSG_METADATA_PTR as usize,
                MSG_PTR_PTR as usize,
                STR_PTR_PTR as usize,
                STR_LEN_PTR as usize,
            )
        };

        Self {
            js_interpreter,
            batch: Batch::default(),
        }
    }
}

impl MsgChannel {
    /// IMPORTANT: Unlike other methods this method is exicuted immediatly and does not wait for the next flush
    pub fn set_node(&mut self, id: NodeId, node: Node) {
        self.js_interpreter.SetNode(id.0, node);
    }

    /// IMPORTANT: Unlike other methods this method is exicuted immediatly and does not wait for the next flush
    pub fn get_node(&mut self, id: NodeId) -> Node {
        self.js_interpreter.GetNode(id.0)
    }

    /// Exicutes any queued operations in the order they were added
    pub fn flush(&mut self) {
        self.batch.encode_op(Op::Stop);
        run_batch(&self.batch.msg, &self.batch.str_buf);
        self.batch.msg.clear();
        self.batch.current_op_batch_idx = 0;
        self.batch.current_op_byte_idx = 3;
        self.batch.str_buf.clear();
    }

    /// Appends a number of nodes as children of the given node.
    pub fn append_child(&mut self, root: MaybeId, child: NodeId) {
        self.batch.append_child(root, child)
    }

    /// Replace a node with another node
    pub fn replace_with(&mut self, root: MaybeId, node: NodeId) {
        self.batch.replace_with(root, node)
    }

    /// Insert a single node after a given node.
    pub fn insert_after(&mut self, root: MaybeId, node: NodeId) {
        self.batch.insert_after(root, node)
    }

    /// Insert a single node before a given node.
    pub fn insert_before(&mut self, root: MaybeId, node: NodeId) {
        self.batch.insert_before(root, node)
    }

    /// Remove a node from the DOM.
    pub fn remove(&mut self, id: MaybeId) {
        self.batch.remove(id)
    }

    /// Create a new text node
    pub fn create_text_node(&mut self, text: impl WritableText, id: MaybeId) {
        self.batch.create_text_node(text, id)
    }

    /// Create a new element node
    pub fn create_element<'a, 'b>(&mut self, tag: impl IntoElement<'a, 'b>, id: Option<NodeId>) {
        self.batch.create_element(tag, id)
    }

    /// Set the textcontent of a node.
    pub fn set_text(&mut self, text: impl WritableText, root: MaybeId) {
        self.batch.set_text(text, root)
    }

    /// Set the value of a node's attribute.
    pub fn set_attribute<'a, 'b>(
        &mut self,
        attr: impl IntoAttribue<'a, 'b>,
        value: impl WritableText,
        root: MaybeId,
    ) {
        self.batch.set_attribute(attr, value, root)
    }

    /// Remove an attribute from a node.
    pub fn remove_attribute<'a, 'b>(&mut self, attr: impl IntoAttribue<'a, 'b>, root: MaybeId) {
        self.batch.remove_attribute(attr, root)
    }

    /// Clone a node and store it with a new id.
    pub fn clone_node(&mut self, id: MaybeId, new_id: MaybeId) {
        self.batch.clone_node(id, new_id)
    }

    /// Move the last node to the first child
    pub fn first_child(&mut self) {
        self.batch.first_child()
    }

    /// Move the last node to the next sibling
    pub fn next_sibling(&mut self) {
        self.batch.next_sibling()
    }

    /// Move the last node to the parent node
    pub fn parent_node(&mut self) {
        self.batch.parent_node()
    }

    /// Store the last node with the given id. This is useful when traversing the document tree.
    pub fn store_with_id(&mut self, id: NodeId) {
        self.batch.store_with_id(id)
    }

    /// Set the last node to the given id. The last node can be used to traverse the document tree without passing objects between wasm and js every time.
    pub fn set_last_node(&mut self, id: NodeId) {
        self.batch.set_last_node(id)
    }

    /// Build a full element, slightly more efficent than creating the element creating the element with `create_element` and then setting the attributes.
    pub fn build_full_element(&mut self, el: ElementBuilder) {
        self.batch.build_full_element(el)
    }

    /// Set a style property on a node.
    pub fn set_style(&mut self, style: &str, value: &str, id: MaybeId) {
        self.batch.set_style(style, value, id)
    }

    /// Remove a style property from a node.
    pub fn remove_style(&mut self, style: &str, id: MaybeId) {
        self.batch.remove_style(style, id)
    }

    /// Adds a batch of operations to the current batch.
    pub fn append(&mut self, batch: Batch) {
        self.batch.append(batch);
    }

    /// Run a batch of operations on the DOM immediately. This only runs the operations that are in the batch, not the operations that are queued in the [`MsgChannel`].
    pub fn run_batch(&mut self, batch: &FinalizedBatch) {
        run_batch(&batch.msg, &batch.str);
    }
}

fn run_batch(msg: &[u8], str_buf: &[u8]) {
    debug_assert_eq!(0usize.to_le_bytes().len(), 32 / 8);
    let msg_ptr = msg.as_ptr() as usize;
    let str_ptr = str_buf.as_ptr() as usize;
    // the pointer will only be updated when the message vec is resized, so we have a flag to check if the pointer has changed to avoid unnecessary decoding
    if unsafe { *MSG_METADATA_PTR } == 255 {
        // this is the first message, so we need to encode all the metadata
        unsafe {
            let mut_ptr_ptr: *mut usize = std::mem::transmute(MSG_PTR_PTR);
            *mut_ptr_ptr = msg_ptr;
            let mut_metadata_ptr: *mut u8 = std::mem::transmute(MSG_METADATA_PTR);
            // the first bit encodes if the msg pointer has changed
            *mut_metadata_ptr = 1;
            let mut_str_ptr_ptr: *mut usize = std::mem::transmute(STR_PTR_PTR);
            *mut_str_ptr_ptr = str_ptr as usize;
            // the second bit encodes if the str pointer has changed
            *mut_metadata_ptr |= 2;
        }
    } else {
        if unsafe { *MSG_PTR_PTR } != msg_ptr {
            unsafe {
                let mut_ptr_ptr: *mut usize = std::mem::transmute(MSG_PTR_PTR);
                *mut_ptr_ptr = msg_ptr;
                let mut_ptr_ptr: *mut u8 = std::mem::transmute(MSG_METADATA_PTR);
                // the first bit encodes if the msg pointer has changed
                *mut_ptr_ptr = 1;
            }
        } else {
            unsafe {
                let mut_ptr_ptr: *mut u8 = std::mem::transmute(MSG_METADATA_PTR);
                // the first bit encodes if the msg pointer has changed
                *mut_ptr_ptr = 0;
            }
        }
        if unsafe { *STR_PTR_PTR } != str_ptr {
            unsafe {
                let mut_str_ptr_ptr: *mut usize = std::mem::transmute(STR_PTR_PTR);
                *mut_str_ptr_ptr = str_ptr as usize;
                let mut_metadata_ptr: *mut u8 = std::mem::transmute(MSG_METADATA_PTR);
                // the second bit encodes if the str pointer has changed
                *mut_metadata_ptr |= 1 << 1;
            }
        }
    }
    unsafe {
        let mut_metadata_ptr: *mut u8 = std::mem::transmute(MSG_METADATA_PTR);
        if !str_buf.is_empty() {
            // the third bit encodes if there is any strings
            *mut_metadata_ptr |= 1 << 2;
            let mut_str_len_ptr: *mut usize = std::mem::transmute(STR_LEN_PTR);
            *mut_str_len_ptr = str_buf.len() as usize;
            if *mut_str_len_ptr < 100 {
                // the fourth bit encodes if the strings are entirely ascii and small
                *mut_metadata_ptr |= (str_buf.is_ascii() as u8) << 3;
            }
        }
    }
    if last_needs_memory() {
        update_last_memory(wasm_bindgen::memory());
    }
    work_last_created();
}

/// Something that can be written as a utf-8 string to a buffer
pub trait WritableText {
    fn write_as_text(self, to: &mut Vec<u8>);
}

impl WritableText for char {
    fn write_as_text(self, to: &mut Vec<u8>) {
        to.push(self as u8);
    }
}

impl<'a> WritableText for &'a str {
    #[inline(always)]
    fn write_as_text(self, to: &mut Vec<u8>) {
        let len = self.len();
        to.reserve(len);
        let old_len = to.len();
        #[allow(clippy::uninit_vec)]
        unsafe {
            let ptr = to.as_mut_ptr();
            let bytes = self.as_bytes();
            let str_ptr = bytes.as_ptr();
            for o in 0..len {
                *ptr.add(old_len + o) = *str_ptr.add(o);
            }
            to.set_len(old_len + len);
        }
        // let _ = to.write(self.as_bytes());
    }
}

impl WritableText for Arguments<'_> {
    fn write_as_text(self, to: &mut Vec<u8>) {
        let _ = to.write_fmt(self);
    }
}

impl<F> WritableText for F
where
    F: FnOnce(&mut Vec<u8>),
{
    fn write_as_text(self, to: &mut Vec<u8>) {
        self(to);
    }
}

macro_rules! write_unsized {
    ($t: ty) => {
        impl WritableText for $t {
            fn write_as_text(self, to: &mut Vec<u8>) {
                let mut n = self;
                let mut n2 = n;
                let mut num_digits = 0;
                while n2 > 0 {
                    n2 /= 10;
                    num_digits += 1;
                }
                let len = num_digits;
                to.reserve(len);
                let ptr = to.as_mut_ptr().cast::<u8>();
                let old_len = to.len();
                let mut i = len - 1;
                loop {
                    unsafe { ptr.add(old_len + i).write((n % 10) as u8 + b'0') }
                    n /= 10;

                    if n == 0 {
                        break;
                    } else {
                        i -= 1;
                    }
                }

                #[allow(clippy::uninit_vec)]
                unsafe {
                    to.set_len(old_len + (len - i));
                }
            }
        }
    };
}

macro_rules! write_sized {
    ($t: ty) => {
        impl WritableText for $t {
            fn write_as_text(self, to: &mut Vec<u8>) {
                let neg = self < 0;
                let mut n = if neg {
                    match self.checked_abs() {
                        Some(n) => n,
                        None => <$t>::MAX / 2 + 1,
                    }
                } else {
                    self
                };
                let mut n2 = n;
                let mut num_digits = 0;
                while n2 > 0 {
                    n2 /= 10;
                    num_digits += 1;
                }
                let len = if neg { num_digits + 1 } else { num_digits };
                to.reserve(len);
                let ptr = to.as_mut_ptr().cast::<u8>();
                let old_len = to.len();
                let mut i = len - 1;
                loop {
                    unsafe { ptr.add(old_len + i).write((n % 10) as u8 + b'0') }
                    n /= 10;

                    if n == 0 {
                        break;
                    } else {
                        i -= 1;
                    }
                }

                if neg {
                    i -= 1;
                    unsafe { ptr.add(i).write(b'-') }
                }

                #[allow(clippy::uninit_vec)]
                unsafe {
                    to.set_len(old_len + (len - i));
                }
            }
        }
    };
}

write_unsized!(u8);
write_unsized!(u16);
write_unsized!(u32);
write_unsized!(u64);
write_unsized!(u128);
write_unsized!(usize);

write_sized!(i8);
write_sized!(i16);
write_sized!(i32);
write_sized!(i64);
write_sized!(i128);
write_sized!(isize);
