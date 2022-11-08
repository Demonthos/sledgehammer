use crate::{builder::MaybeId, ElementBuilder, IntoAttribue, IntoElement, NodeId, WritableText};

// operations that have no booleans can be encoded as a half byte, these are placed first
pub(crate) enum Op {
    /// Navigates to the last node to the first child of the current node.
    FirstChild = 0,

    /// Navigates to the last node to the last child of the current node.
    NextSibling = 1,

    /// Navigates to the last node to the parent of the current node.
    ParentNode = 2,

    /// Stores the last node with a new id.
    StoreWithId = 3,

    /// Manually set the last node.
    SetLastNode = 4,

    /// Stop
    Stop = 5,

    /// Build Full Element
    BuildFullElement = 6,

    /// Pop the topmost node from our stack and append them to the node
    AppendChildren = 7,

    /// Replace a given (single) node with a handful of nodes currently on the stack.
    ReplaceWith = 8,

    /// Insert a number of nodes after a given node.
    InsertAfter = 9,

    /// Insert a number of nodes before a given node.
    InsertBefore = 10,

    /// Remove a particular node from the DOM
    Remove = 11,

    /// Create a new text node
    CreateTextNode = 12,

    /// Create a new element node
    CreateElement = 13,

    /// Set the textcontent of a node.
    SetText = 14,

    /// Set the value of a node's attribute.
    SetAttribute = 15,

    /// Remove an attribute from a node.
    RemoveAttribute = 16,

    /// Set a style property on a node.
    SetStyle = 17,

    /// Remove a style property from a node.
    RemoveStyle = 18,

    /// Clones a node.
    CloneNode = 19,

    /// Clones the children of a node. (allows cloning fragments)
    CloneNodeChildren = 20,

    /// Does nothing, but allows us to skip a byte.
    NoOp = 21,
}

pub struct FinalizedBatch {
    pub(crate) msg: Vec<u8>,
    pub(crate) str: Vec<u8>,
}

pub struct Batch {
    pub(crate) msg: Vec<u8>,
    pub(crate) str_buf: Vec<u8>,
    pub(crate) current_op_batch_idx: usize,
    pub(crate) current_op_byte_idx: usize,
    pub(crate) current_op_bit_pack_index: u8,
}

impl Default for Batch {
    fn default() -> Self {
        Self {
            msg: Vec::new(),
            str_buf: Vec::new(),
            current_op_byte_idx: 3,
            current_op_bit_pack_index: 0,
            current_op_batch_idx: 0,
        }
    }
}

impl Batch {
    /// Finalizes the batch.
    pub fn finalize(mut self) -> FinalizedBatch {
        self.encode_op(Op::Stop);
        FinalizedBatch {
            msg: self.msg,
            str: self.str_buf,
        }
    }

    /// Appends a number of nodes as children of the given node.
    pub fn append_child(&mut self, root: MaybeId, child: NodeId) {
        self.encode_op(Op::AppendChildren);
        let size = root.encoded_size() + 4;
        self.msg.reserve(size as usize);
        unsafe {
            self.encode_maybe_id_prealloc(root);
            self.encode_bool(false);
            self.encode_id_prealloc(child);
        }
    }

    /// Appends a number of nodes as children of the given node.
    pub fn append_children(&mut self, root: MaybeId, children: Vec<NodeId>) {
        self.encode_op(Op::AppendChildren);
        self.encode_maybe_id(root);
        self.encode_bool(true);
        self.encode_u32(children.len() as u32);
        for child in children {
            self.encode_id(child);
        }
    }

    /// Replace a node with another node
    pub fn replace_with(&mut self, root: MaybeId, node: NodeId) {
        self.encode_op(Op::ReplaceWith);
        let size = root.encoded_size() + 4;
        self.msg.reserve(size as usize);
        unsafe {
            self.encode_maybe_id_prealloc(root);
            self.encode_bool(false);
            self.encode_id_prealloc(node);
        }
    }

    /// Replace a node with a number of nodes
    pub fn replace_with_nodes(&mut self, root: MaybeId, nodes: Vec<NodeId>) {
        self.encode_op(Op::ReplaceWith);
        self.encode_maybe_id(root);
        self.encode_bool(true);
        self.encode_u32(nodes.len() as u32);
        for node in nodes {
            self.encode_id(node);
        }
    }

    /// Insert a single node after a given node.
    pub fn insert_after(&mut self, root: MaybeId, node: NodeId) {
        self.encode_op(Op::InsertAfter);
        let size = root.encoded_size() + 4;
        self.msg.reserve(size as usize);
        unsafe {
            self.encode_maybe_id_prealloc(root);
            self.encode_bool(false);
            self.encode_id_prealloc(node);
        }
    }

    /// Insert a number of nodes after a given node.
    pub fn insert_nodes_after(&mut self, root: MaybeId, nodes: &[NodeId]) {
        self.encode_op(Op::InsertAfter);
        self.encode_maybe_id(root);
        self.encode_bool(true);
        self.encode_u32(nodes.len() as u32);
        for node in nodes {
            self.encode_id(*node);
        }
    }

    /// Insert a single node before a given node.
    pub fn insert_before(&mut self, root: MaybeId, node: NodeId) {
        self.encode_op(Op::InsertBefore);
        let size = root.encoded_size() + 4;
        self.msg.reserve(size as usize);
        unsafe {
            self.encode_maybe_id_prealloc(root);
            self.encode_bool(false);
            self.encode_id_prealloc(node);
        }
    }

    /// Insert a number of nodes before a given node.
    pub fn insert_nodes_before(&mut self, root: MaybeId, nodes: &[NodeId]) {
        self.encode_op(Op::InsertBefore);
        self.encode_maybe_id(root);
        self.encode_bool(true);
        self.encode_u32(nodes.len() as u32);
        for node in nodes {
            self.encode_id(*node);
        }
    }

    /// Remove a node from the DOM.
    pub fn remove(&mut self, id: MaybeId) {
        self.encode_op(Op::Remove);
        self.encode_maybe_id(id);
    }

    /// Create a new text node
    pub fn create_text_node(&mut self, text: impl WritableText, id: MaybeId) {
        self.encode_op(Op::CreateTextNode);
        let size = id.encoded_size() + 2;
        self.msg.reserve(size as usize);
        unsafe {
            self.encode_str_prealloc(text);
            self.encode_maybe_id_prealloc(id);
        }
    }

    /// Create a new element node
    pub fn create_element<'a, 'b, E>(&mut self, tag: E, id: Option<NodeId>)
    where
        E: IntoElement<'a, 'b>,
    {
        self.encode_op(Op::CreateElement);
        self.msg
            .reserve((E::SINGLE_BYTE as u8 + (id.is_some() as u8) * 4) as usize);
        unsafe {
            tag.encode_prealloc(self);
            self.encode_optional_id_prealloc(id);
        }
    }

    /// Set the textcontent of a node.
    pub fn set_text(&mut self, text: impl WritableText, root: MaybeId) {
        self.encode_op(Op::SetText);
        let size = root.encoded_size() + 2;
        self.msg.reserve(size as usize);
        unsafe {
            self.encode_maybe_id_prealloc(root);
            self.encode_str_prealloc(text);
        }
    }

    /// Set the value of a node's attribute.
    #[inline(never)]
    pub fn set_attribute<'a, 'b, A>(&mut self, attr: A, value: impl WritableText, root: MaybeId)
    where
        A: IntoAttribue<'a, 'b>,
    {
        self.encode_op(Op::SetAttribute);
        self.msg
            .reserve((A::SINGLE_BYTE as u8 + root.encoded_size() + 2) as usize);
        unsafe {
            self.encode_maybe_id_prealloc(root);
            attr.encode_prealloc(self);
            self.encode_str_prealloc(value);
        }
    }

    /// Remove an attribute from a node.
    pub fn remove_attribute<'a, 'b, A>(&mut self, attr: A, root: MaybeId)
    where
        A: IntoAttribue<'a, 'b>,
    {
        self.encode_op(Op::RemoveAttribute);
        let size = A::SINGLE_BYTE as u8 + root.encoded_size();
        self.msg.reserve(size as usize);
        unsafe {
            self.encode_maybe_id_prealloc(root);
            attr.encode_prealloc(self);
        }
    }

    /// Clone a node and store it with a new id.
    pub fn clone_node(&mut self, id: MaybeId, new_id: MaybeId) {
        self.encode_op(Op::CloneNode);
        let size = id.encoded_size() + new_id.encoded_size();
        self.msg.reserve(size as usize);
        self.encode_maybe_id(id);
        self.encode_maybe_id(new_id);
    }

    /// Clone the children of a given node and store them with new ids.
    pub fn clone_node_children(&mut self, id: MaybeId, new_ids: Vec<NodeId>) {
        self.encode_op(Op::CloneNodeChildren);
        self.encode_maybe_id(id);
        for id in new_ids {
            self.encode_optional_id_with_byte_bool(Some(id));
        }
    }

    /// Move the last node to the first child
    pub fn first_child(&mut self) {
        self.encode_op(Op::FirstChild);
    }

    /// Move the last node to the next sibling
    pub fn next_sibling(&mut self) {
        self.encode_op(Op::NextSibling);
    }

    /// Move the last node to the parent node
    pub fn parent_node(&mut self) {
        self.encode_op(Op::ParentNode);
    }

    /// Store the last node with the given id. This is useful when traversing the document tree.
    pub fn store_with_id(&mut self, id: NodeId) {
        self.encode_op(Op::StoreWithId);
        self.encode_id(id);
    }

    /// Set the last node to the given id. The last node can be used to traverse the document tree without passing objects between wasm and js every time.
    pub fn set_last_node(&mut self, id: NodeId) {
        self.encode_op(Op::SetLastNode);
        self.encode_id(id);
    }

    /// Build a full element, slightly more efficent than creating the element creating the element with `create_element` and then setting the attributes.
    pub fn build_full_element(&mut self, el: ElementBuilder) {
        self.encode_op(Op::BuildFullElement);
        el.encode(self);
    }

    /// Set a style property on a node.
    pub fn set_style(&mut self, style: &str, value: &str, id: MaybeId) {
        self.encode_op(Op::SetStyle);
        let size = id.encoded_size() + 2 + 2;
        self.msg.reserve(size as usize);
        self.encode_maybe_id(id);
        unsafe {
            self.encode_str_prealloc(style);
            self.encode_str_prealloc(value);
        }
    }

    /// Remove a style property from a node.
    pub fn remove_style(&mut self, style: &str, id: MaybeId) {
        self.encode_op(Op::RemoveStyle);
        let size = id.encoded_size() + 2;
        self.msg.reserve(size as usize);
        unsafe {
            self.encode_maybe_id_prealloc(id);
            self.encode_str_prealloc(style);
        }
    }

    #[inline]
    pub(crate) unsafe fn encode_optional_id_prealloc(&mut self, id: Option<NodeId>) {
        match id {
            Some(id) => {
                self.encode_bool(true);
                self.encode_id_prealloc(id);
            }
            None => {
                self.encode_bool(false);
            }
        }
    }

    #[inline]
    pub(crate) unsafe fn encode_maybe_id_prealloc(&mut self, id: MaybeId) {
        match id {
            MaybeId::Node(id) => {
                self.encode_bool(true);
                self.encode_id_prealloc(id);
            }
            MaybeId::LastNode => {
                self.encode_bool(false);
            }
        }
    }

    #[inline]
    pub(crate) fn encode_maybe_id(&mut self, id: MaybeId) {
        match id {
            MaybeId::Node(id) => {
                self.encode_bool(true);
                self.encode_id(id);
            }
            MaybeId::LastNode => {
                self.encode_bool(false);
            }
        }
    }

    #[inline]
    pub(crate) fn encode_optional_id_with_byte_bool(&mut self, id: Option<NodeId>) {
        match id {
            Some(id) => {
                self.msg.push(1);
                self.encode_id(id);
            }
            None => {
                self.msg.push(0);
            }
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn encode_id_prealloc(&mut self, id: NodeId) {
        self.encode_u32_prealloc(id.0);
    }

    #[inline(always)]
    pub(crate) fn encode_id(&mut self, id: NodeId) {
        self.encode_u32(id.0);
    }

    #[inline(always)]
    pub(crate) fn encode_u32(&mut self, val: u32) {
        self.msg.reserve(4);
        unsafe {
            self.encode_u32_prealloc(val);
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn encode_u32_prealloc(&mut self, val: u32) {
        let le = val.to_le();
        unsafe {
            let len = self.msg.len();
            self.msg.as_mut_ptr().add(len).cast::<u32>().write(le);
            self.msg.set_len(len + 4);
        }
    }

    #[inline(always)]
    pub(crate) fn encode_u16(&mut self, val: u16) {
        self.msg.reserve(2);
        unsafe {
            self.encode_u16_prealloc(val);
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn encode_u16_prealloc(&mut self, val: u16) {
        let le = val.to_le();
        #[allow(clippy::uninit_vec)]
        unsafe {
            let len = self.msg.len();
            self.msg.as_mut_ptr().add(len).cast::<u16>().write(le);
            self.msg.set_len(len + 2);
        }
    }

    #[inline]
    pub(crate) fn encode_str(&mut self, string: impl WritableText) {
        let prev_len = self.str_buf.len();
        string.write_as_text(&mut self.str_buf);
        let len = self.str_buf.len() - prev_len;
        self.encode_u16(len as u16);
    }

    #[inline]
    pub(crate) unsafe fn encode_str_prealloc(&mut self, string: impl WritableText) {
        let prev_len = self.str_buf.len();
        string.write_as_text(&mut self.str_buf);
        let len = self.str_buf.len() - prev_len;
        self.encode_u16_prealloc(len as u16);
    }

    #[inline]
    pub(crate) fn encode_cachable_str(&mut self, string: impl WritableText) {
        let prev_len = self.str_buf.len();
        string.write_as_text(&mut self.str_buf);
        let len = self.str_buf.len() - prev_len;
        self.encode_u16(len as u16);
    }

    #[inline]
    pub(crate) fn encode_op(&mut self, op: Op) {
        let u8_op = op as u8;

        self.current_op_byte_idx += 1;
        if self.current_op_byte_idx - self.current_op_batch_idx < 4 {
            unsafe {
                *self.msg.get_unchecked_mut(self.current_op_byte_idx) = u8_op;
            }
        } else {
            self.current_op_batch_idx = self.msg.len();
            self.current_op_byte_idx = self.current_op_batch_idx;
            // reserve four bytes for the op batch
            #[allow(clippy::uninit_vec)]
            unsafe {
                let len = self.msg.len();
                self.msg.reserve(4);
                self.msg.set_len(len + 4);
                *self.msg.get_unchecked_mut(self.current_op_batch_idx) = u8_op;
            }
        }
        self.current_op_bit_pack_index = 0;
    }

    #[inline]
    pub(crate) fn encode_bool(&mut self, value: bool) {
        if self.current_op_bit_pack_index < 3 {
            if value {
                unsafe {
                    *self.msg.get_unchecked_mut(self.current_op_byte_idx) |=
                        1 << (self.current_op_bit_pack_index + 5);
                }
            }
            self.current_op_bit_pack_index += 1;
        } else {
            todo!("handle more than 3 bools in a op");
        }
    }

    pub(crate) fn append(&mut self, mut batch: Self) {
        // add empty operations to the batch to make sure the batch is aligned
        let operations_left = 3 - (self.current_op_byte_idx - self.current_op_batch_idx);
        for _ in 0..operations_left {
            self.encode_op(Op::NoOp);
        }

        self.current_op_byte_idx = self.msg.len() + batch.current_op_byte_idx;
        self.current_op_batch_idx = self.msg.len() + batch.current_op_batch_idx;
        self.current_op_bit_pack_index = batch.current_op_bit_pack_index;
        self.str_buf.extend_from_slice(&batch.str_buf);
        self.msg.append(&mut batch.msg);
    }
}
