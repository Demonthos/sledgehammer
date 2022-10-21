use std::{fmt::Arguments, io::Write};

use web_sys::{Element, Node};

use crate::{
    get_id_size, last_needs_memory, set_id_size, update_last_memory, work_last_created,
    ElementBuilderExt, IntoAttribue, JsInterpreter, MSG_POS_UPDATED_PTR, MSG_PTR_PTR, STR_LEN_PTR,
    STR_PTR_PTR,
};

/// An id that may be either the last node or a node with an assigned id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaybeId {
    /// The last node that was created or navigated to.
    LastNode,
    /// A node that was created and stored with an id
    Node(NodeId),
}

/// A node that was created and stored with an id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    pub(crate) const fn to_le_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

pub(crate) fn id_size(bytes: [u8; 4]) -> u8 {
    let first_contentful_byte = bytes.iter().rev().position(|&b| b != 0).unwrap_or(4);
    (4 - first_contentful_byte) as u8
}

#[allow(clippy::len_without_is_empty)]
pub trait VecLike: AsRef<[u8]> + Write {
    fn add_element(&mut self, element: u8);

    #[inline]
    fn extend_owned_slice<const N: usize>(&mut self, slice: [u8; N]) {
        self.extend_slice(&slice)
    }

    fn extend_slice(&mut self, slice: &[u8]);

    fn len(&self) -> usize;

    fn clear(&mut self);

    fn set(&mut self, index: usize, value: u8);
}

impl VecLike for Vec<u8> {
    fn add_element(&mut self, element: u8) {
        self.push(element);
    }

    fn extend_slice(&mut self, slice: &[u8]) {
        self.extend(slice.iter().copied());
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn set(&mut self, index: usize, value: u8) {
        self[index] = value;
    }
}

/// A channel to send batched messages to the interpreter.
/// IMPORTANT: All of the functions that are not marked otherwise are qued and not exicuted imidately. When you want to exicute the que you have to call `flush`.
pub struct MsgChannel<V: VecLike + AsRef<[u8]> = Vec<u8>> {
    pub(crate) msg: V,
    pub(crate) str_buf: V,
    pub(crate) js_interpreter: JsInterpreter,
}

impl<V: VecLike + AsRef<[u8]>> MsgChannel<V> {
    pub fn with(v: V, v2: V, el: Element) -> Self {
        format!(
            "init: {:?}, {:?}, {:?}",
            unsafe { MSG_PTR_PTR as usize },
            unsafe { STR_PTR_PTR as usize },
            unsafe { STR_LEN_PTR as usize }
        );
        let js_interpreter = unsafe {
            JsInterpreter::new(
                el,
                wasm_bindgen::memory(),
                MSG_POS_UPDATED_PTR as usize,
                MSG_PTR_PTR as usize,
                STR_PTR_PTR as usize,
                STR_LEN_PTR as usize,
            )
        };

        Self {
            msg: v,
            str_buf: v2,
            js_interpreter,
        }
    }
}

impl MsgChannel<Vec<u8>> {
    pub fn new(el: Element) -> Self {
        Self::with(Vec::new(), Vec::new(), el)
    }
}

enum Op {
    /// Pop the topmost node from our stack and append them to the node
    AppendChildren = 0,

    /// Replace a given (single) node with a handful of nodes currently on the stack.
    ReplaceWith = 1,

    /// Insert a number of nodes after a given node.
    InsertAfter = 2,

    /// Insert a number of nodes before a given node.
    InsertBefore = 3,

    /// Remove a particular node from the DOM
    Remove = 4,

    /// Create a new text node
    CreateTextNode = 5,

    /// Create a new element node
    CreateElement = 6,

    /// Set the textcontent of a node.
    SetText = 10,

    /// Set the value of a node's attribute.
    SetAttribute = 11,

    /// Remove an attribute from a node.
    RemoveAttribute = 12,

    /// Clones a node.
    CloneNode = 13,

    /// Clones the children of a node. (allows cloning fragments)
    CloneNodeChildren = 14,

    /// Navigates to the last node to the first child of the current node.
    FirstChild = 15,

    /// Navigates to the last node to the last child of the current node.
    NextSibling = 16,

    /// Navigates to the last node to the parent of the current node.
    ParentNode = 17,

    /// Stores the last node with a new id.
    StoreWithId = 18,

    /// Manually set the last node.
    SetLastNode = 19,

    /// Set id size
    SetIdSize = 20,

    /// Stop
    Stop = 21,

    /// Build Full Element
    BuildFullElement = 22,
}

impl<V: VecLike> MsgChannel<V> {
    /// Appends a number of nodes as children of the given node.
    pub fn append_children(&mut self, root: MaybeId, children: Vec<NodeId>) {
        let root = self.check_maybe_id(root);
        for child in &children {
            self.check_id(*child);
        }
        self.msg.add_element(Op::AppendChildren as u8);
        self.encode_maybe_id(root);
        self.msg
            .extend_slice(&(children.len() as u32).to_le_bytes());
        for child in children {
            self.encode_id(child.to_le_bytes());
        }
    }

    /// Replace a given (single) node with a number of nodes
    pub fn replace_with(&mut self, root: MaybeId, nodes: Vec<NodeId>) {
        let root = self.check_maybe_id(root);
        for child in &nodes {
            self.check_id(*child);
        }
        self.msg.add_element(Op::ReplaceWith as u8);
        self.encode_maybe_id(root);
        self.msg.extend_slice(&(nodes.len() as u32).to_le_bytes());
        for node in nodes {
            self.encode_id(node.to_le_bytes());
        }
    }

    /// Insert a number of nodes after a given node.
    pub fn insert_after(&mut self, root: MaybeId, nodes: Vec<NodeId>) {
        let root = self.check_maybe_id(root);
        for child in &nodes {
            self.check_id(*child);
        }
        self.msg.add_element(Op::InsertAfter as u8);
        self.encode_maybe_id(root);
        self.msg.extend_slice(&(nodes.len() as u32).to_le_bytes());
        for node in nodes {
            self.encode_id(node.to_le_bytes());
        }
    }

    /// Insert a number of nodes before a given node.
    pub fn insert_before(&mut self, root: MaybeId, nodes: Vec<NodeId>) {
        let root = self.check_maybe_id(root);
        for child in &nodes {
            self.check_id(*child);
        }
        self.msg.add_element(Op::InsertBefore as u8);
        self.encode_maybe_id(root);
        self.msg.extend_slice(&(nodes.len() as u32).to_le_bytes());
        for node in nodes {
            self.encode_id(node.to_le_bytes());
        }
    }

    /// Remove a node from the DOM.
    pub fn remove(&mut self, id: MaybeId) {
        let root = self.check_maybe_id(id);
        self.msg.add_element(Op::Remove as u8);
        self.encode_maybe_id(root);
    }

    /// Create a new text node
    pub fn create_text_node(&mut self, text: Arguments, id: MaybeId) {
        let root = self.check_maybe_id(id);
        self.msg.add_element(Op::CreateTextNode as u8);
        self.encode_str(text);
        self.encode_maybe_id(root);
    }

    /// Create a new element node
    pub fn create_element(&mut self, tag: Arguments, ns: Option<Arguments>, id: MaybeId) {
        let root = self.check_maybe_id(id);
        self.msg.add_element(Op::CreateElement as u8);
        self.encode_cachable_str(tag);
        if let Some(ns) = ns {
            self.msg.add_element(1);
            self.encode_cachable_str(ns);
        } else {
            self.msg.add_element(0);
        }
        self.encode_maybe_id(root);
    }

    /// Set the textcontent of a node.
    pub fn set_text(&mut self, text: Arguments, root: MaybeId) {
        let root = self.check_maybe_id(root);
        self.msg.add_element(Op::SetText as u8);
        self.encode_maybe_id(root);
        self.encode_str(text);
    }

    /// Set the value of a node's attribute.
    pub fn set_attribute(&mut self, attr: impl IntoAttribue, value: Arguments, root: MaybeId) {
        let root = self.check_maybe_id(root);
        self.msg.add_element(Op::SetAttribute as u8);
        self.encode_maybe_id(root);
        attr.encode(self);
        self.encode_str(value);
    }

    /// Remove an attribute from a node.
    pub fn remove_attribute(&mut self, attr: impl IntoAttribue, root: MaybeId) {
        let root = self.check_maybe_id(root);
        self.msg.add_element(Op::RemoveAttribute as u8);
        self.encode_maybe_id(root);
        attr.encode(self);
    }

    /// Clone a node and store it with a new id.
    pub fn clone_node(&mut self, id: MaybeId, new_id: MaybeId) {
        let root = self.check_maybe_id(id);
        let new_id = self.check_maybe_id(new_id);
        self.msg.add_element(Op::CloneNode as u8);
        self.encode_maybe_id(root);
        self.encode_maybe_id(new_id);
    }

    /// Clone the children of a given node and store them with new ids.
    pub fn clone_node_children(&mut self, id: MaybeId, new_ids: Vec<NodeId>) {
        let root = self.check_maybe_id(id);
        for id in &new_ids {
            self.check_id(*id);
        }
        self.msg.add_element(Op::CloneNodeChildren as u8);
        self.encode_maybe_id(root);
        for id in new_ids {
            self.encode_maybe_id(Some(id.to_le_bytes()));
        }
    }

    /// Move the last node to the first child
    pub fn first_child(&mut self) {
        self.msg.add_element(Op::FirstChild as u8);
    }

    /// Move the last node to the next sibling
    pub fn next_sibling(&mut self) {
        self.msg.add_element(Op::NextSibling as u8);
    }

    /// Move the last node to the parent node
    pub fn parent_node(&mut self) {
        self.msg.add_element(Op::ParentNode as u8);
    }

    /// Store the last node with the given id. This is useful when traversing the document tree.
    pub fn store_with_id(&mut self, id: NodeId) {
        let id = self.check_id(id);
        self.msg.add_element(Op::StoreWithId as u8);
        self.encode_id(id);
    }

    /// Set the last node to the given id. The last node can be used to traverse the document tree without passing objects between wasm and js every time.
    pub fn set_last_node(&mut self, id: NodeId) {
        let id = self.check_id(id);
        self.msg.add_element(Op::SetLastNode as u8);
        self.encode_id(id);
    }

    /// Build a full element, slightly more efficent than creating the element creating the element with `create_element` and then setting the attributes.
    pub fn build_full_element(&mut self, el: impl ElementBuilderExt) {
        self.msg.add_element(Op::BuildFullElement as u8);
        el.encode(self, get_id_size());
    }

    #[inline]
    pub(crate) fn encode_maybe_id(&mut self, id: Option<[u8; 4]>) {
        match id {
            Some(id) => {
                self.msg.add_element(1);
                self.encode_id(id);
            }
            None => {
                self.msg.add_element(0);
            }
        }
    }

    #[inline]
    pub(crate) fn encode_id(&mut self, bytes: [u8; 4]) {
        self.msg.extend_slice(&bytes[..(get_id_size() as usize)]);
    }

    #[inline]
    fn check_id(&mut self, id: NodeId) -> [u8; 4] {
        let bytes = id.0.to_le_bytes();
        let byte_size = id_size(bytes);
        if byte_size > get_id_size() {
            self.set_byte_size(byte_size);
        }
        bytes
    }

    #[inline]
    fn check_maybe_id(&mut self, id: MaybeId) -> Option<[u8; 4]> {
        match id {
            MaybeId::Node(id) => Some(self.check_id(id)),
            MaybeId::LastNode => None,
        }
    }

    #[inline]
    fn set_byte_size(&mut self, byte_size: u8) {
        let nearest_larger_power_of_two = match byte_size {
            1 => 1,
            2 => 2,
            3 => 4,
            4 => 4,
            _ => unreachable!(),
        };
        set_id_size(nearest_larger_power_of_two);
        self.msg.add_element(Op::SetIdSize as u8);
        self.msg.add_element(nearest_larger_power_of_two);
    }

    pub(crate) fn encode_str(&mut self, string: Arguments) {
        let prev_len = self.str_buf.len();
        self.str_buf.write_fmt(string).unwrap();
        let len = self.str_buf.len() - prev_len;
        self.msg.extend_slice(&(len as u16).to_le_bytes());
    }

    pub(crate) fn encode_cachable_str(&mut self, string: Arguments) {
        let prev_len = self.str_buf.len();
        self.str_buf.write_fmt(string).unwrap();
        let len = self.str_buf.len() - prev_len;
        self.msg.extend_slice(&(len as u16).to_le_bytes());
    }

    /// Exicutes any queued operations
    #[inline]
    pub fn flush(&mut self) {
        assert_eq!(0usize.to_le_bytes().len(), 32 / 8);
        self.msg.add_element(Op::Stop as u8);
        let msg_ptr = self.msg.as_ref().as_ptr() as usize;
        // the pointer will only be updated when the message vec is resized, so we have a flag to check if the pointer has changed to avoid unnecessary decoding
        if unsafe { *MSG_PTR_PTR } != msg_ptr || unsafe { *MSG_POS_UPDATED_PTR } == 2 {
            unsafe {
                let mut_ptr_ptr: *mut usize = std::mem::transmute(MSG_PTR_PTR);
                *mut_ptr_ptr = msg_ptr;
                let mut_ptr_ptr: *mut usize = std::mem::transmute(MSG_POS_UPDATED_PTR);
                *mut_ptr_ptr = 1;
            }
        } else {
            unsafe {
                let mut_ptr_ptr: *mut usize = std::mem::transmute(MSG_POS_UPDATED_PTR);
                *mut_ptr_ptr = 0;
            }
        }
        unsafe {
            let mut_str_ptr_ptr: *mut usize = std::mem::transmute(STR_PTR_PTR);
            *mut_str_ptr_ptr = self.str_buf.as_ref().as_ptr() as usize;
            let mut_str_len_ptr: *mut usize = std::mem::transmute(STR_LEN_PTR);
            *mut_str_len_ptr = self.str_buf.len() as usize;
        }
        if last_needs_memory() {
            update_last_memory(wasm_bindgen::memory())
        }
        work_last_created();
        self.msg.clear();
        self.str_buf.clear();
    }

    /// IMPORTANT: Unlike other methods this method is exicuted imediately and does not wait for the next flush
    pub fn set_node(&mut self, id: NodeId, node: Node) {
        self.js_interpreter.SetNode(id.0, node);
    }

    /// IMPORTANT: Unlike other methods this method is exicuted imediately and does not wait for the next flush
    pub fn get_node(&mut self, id: NodeId) -> Node {
        self.js_interpreter.GetNode(id.0)
    }
}