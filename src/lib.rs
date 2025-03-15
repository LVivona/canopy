// Copyright 2025 Luca Vince Vivona
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// “Software”), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate alloc;

pub mod error;
mod node;

pub use crate::node::{Node, NodeRef, PrevNodeRef, NodeIter};

#[cfg(test)]
mod tests {
    use crate::{error::NodeError, node::NodeIter, Node, NodeRef};

    fn assert_parent_eq<T>(parent: &NodeRef<T>, expected_parent: &NodeRef<T>) {
        assert!(NodeRef::ptr_eq(
            parent,
            expected_parent
        ));
    }

    #[test]
    fn create_root_node() {
        let node: NodeRef<bool> = Node::parent(true);
        assert!(node.borrow().is_root());
    }

    #[test]
    fn create_leaf_node() -> Result<(), NodeError> {
        let parent = Node::parent(true);
        let node = Node::leaf(true, None);
        
        Node::insert_node(&parent, &node)?;
        #[cfg(not(feature = "std"))]
        let ref_parent = node.borrow().prev().unwrap();
        #[cfg(feature = "std")]
        let ref_parent = node.borrow().prev().unwrap().upgrade().unwrap();
        assert!(node.borrow().is_leaf());
        assert_parent_eq(&ref_parent, &parent);
        Ok(())
    }

    #[test]
    fn root_has_no_children() {
        let root = Node::parent(true);
        assert!(!root.borrow().has_children());
    }

    #[test]
    fn non_root_expectation() {
        let root = Node::parent(true);
        assert!(root.borrow().expect_root().is_err());
    }

    #[test]
    fn node_insertion() -> Result<(), NodeError> {
        let root: NodeRef<u8> = Node::parent(1);
        let _ = Node::insert(&root, 2)?;
        let child = Node::insert(&root, 3)?;
        let _ = Node::insert(&child, 4)?;
        assert_eq!(root.borrow().children().len(), 2);
        assert_eq!(*root.borrow().children()[0].borrow().value(), 2);
        assert_eq!(*root.borrow().children()[1].borrow().value(), 3);
        assert_eq!(*child.borrow().children()[0].borrow().value(), 4);
        Ok(())
    }

    #[test]
    fn pop_node() -> Result<(), NodeError> {
        let root = Node::parent(1);
        let child = Node::insert(&root, 2)?;
        let _ = Node::insert(&root, 4)?;
        assert_eq!(root.borrow().children().len(), 2);
        let _ = Node::pop(&root, &child);
        assert!(child.borrow().prev().is_none());
        assert_eq!(root.borrow().children().len(), 1);
        Ok(())
    }

    #[test]
    fn pop_nonexistent_node() -> Result<(), NodeError> {
        let root = Node::parent(1);
        let _ = Node::insert(&root, 2)?;
        let _ = Node::insert(&root, 4)?;
        let node = Node::leaf(2, None);
        let _ = Node::pop(&root, &node);
        assert_eq!(root.borrow().children().len(), 2);
        assert!(node.borrow().prev().is_none());
        Ok(())
    }

    #[test]
    fn downgrade_failure() {
        let leaf = Node::leaf(42, None);
        assert_eq!(Node::downgrade(&leaf), Err(NodeError::DowngradeNotParent));
    }

    #[test]
    fn downgrade_root_failure() {
        let root = Node::parent(42);
        assert_eq!(
            Node::downgrade(&root),
            Err(NodeError::RootDowngradeNotAllowed)
        );
    }

    #[test]
    fn upgrade_failure_for_parent() {
        let root = Node::parent(42);
        let leaf = Node::leaf(101, Some(root.clone()));
        assert_eq!(
            Node::upgrade(&root, &leaf),
            Err(NodeError::ParentUpgradeNotAllowed)
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn display_node() -> Result<(), NodeError> {
        let root = Node::parent(42);
        let leaf = Node::insert(&root, 420)?;
        println!("{}", root.borrow());
        println!("{}", leaf.borrow());
        Ok(())
    }

    #[test]
    fn insert_node() -> Result<(), NodeError> {
        let root = Node::parent(1);
        let child = Node::insert(&root, 2)?;
        assert!(Node::pop(&root, &child)?);
        let root2 = Node::parent(2);
        Node::insert_node(&root2, &child)?;

        #[cfg(not(feature = "std"))]
        let parent = child.borrow().prev().unwrap();
        #[cfg(feature = "std")]
        let parent = child.borrow().prev().unwrap().upgrade().unwrap();

        assert_eq!(root.borrow().children().len(), 0);
        assert_eq!(root2.borrow().children().len(), 1);
        assert_parent_eq(&parent, &root2);
        Ok(())
    }

    #[test]
    fn iter_test() -> Result<(), NodeError> {
        let root: NodeRef<u8> = Node::parent(1);
        let _: NodeRef<u8> = Node::insert(&root, 2)?;
        let child2: NodeRef<u8> = Node::insert(&root, 3)?;
        let _: NodeRef<u8> = Node::insert(&child2, 4)?;
        let _: NodeRef<u8> = Node::insert(&child2, 5)?;
    
        let mut nodes: NodeIter<u8> = Node::iter(root.clone());
        let mut count = 1; 
        while let Some(node) = nodes.next() {
            // order printed out: 1, 2, 3, 4, 5
            println!("{}", node.borrow().value());
            assert!(node.borrow().value() == &count);
            count += 1;
        }
        Ok(())
        
    }
}
