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

pub use crate::node::{Node, NodeRef};


#[cfg(test)]
mod test {

    use crate::{Node, NodeRef, error::NodeError};

    #[test]
    fn node_parent_creation_root() {
        let node: NodeRef<bool> = Node::parent(true);
        assert!(node.borrow().is_root())
    }


    #[test]
    fn node_parent_creation_leaf() -> Result<(), NodeError> {
        
        let parent = Node::parent(true);
        let node = Node::leaf(true, Some(parent.clone()));
        let root_ref = parent.borrow();
        let node_ref = node.borrow();
        assert!(node_ref.is_leaf() && node_ref.prev().unwrap().borrow().value() == root_ref.value());
        Ok(())
    }

    #[test]
    fn node_root_chidren_has_no_children() {
        let root = Node::parent(true);
        assert!(!root.borrow().has_children())
    }

    #[test]
    fn exception_non_root_check() {
        let root = Node::parent(true);
        assert!(root.borrow().expect_root().is_err())
    }

    #[test]
    fn test_insertion() -> Result<(), NodeError> {
        let root: NodeRef<u8> = Node::parent(1);
        let _ = Node::insert(&root, 2)?;
        let child = Node::insert(&root, 3)?;
        let _ = Node::insert(&child.clone(), 4)?;

        // First, let's check that root has two children
        assert_eq!(root.borrow().children().len(), 2);

        // Then check that the first child's value is 2
        assert_eq!(*root.borrow().children()[0].borrow().value(), 2);

        // Check that the second child's value is 3
        assert_eq!(*root.borrow().children()[1].borrow().value(), 3);

        // Check that the grandchild's value is 4
        assert_eq!(*child.borrow().children()[0].borrow().value(), 4);

        Ok(())
    }

    #[test]
    fn test_pop() -> Result<(), NodeError> {
        let root = Node::parent(1);
        let child = Node::insert(&root, 2)?;
        let _ = Node::insert(&root, 4)?;

        assert!(root.borrow().children().len() == 2);
        let _ = Node::pop(&root, &child);
        assert!(child.borrow().prev().is_none());
        assert!(root.borrow().children().len() == 1); // Successfully removed
        Ok(())
    }

    #[test]
    fn test_pop_non_ref() -> Result<(), NodeError> {
        let root = Node::parent(1);
        let _ = Node::insert(&root, 2)?;
        let _ = Node::insert(&root, 4)?;

        let node = Node::leaf(2, None);
        let _ = Node::pop(&root, &node);
        assert!(root.borrow().children().len() == 2); // Successfully removed
        assert!(node.borrow().prev().is_none()); // Successfully removed
        Ok(())
    }

    #[test]
    fn donwgrade_test() -> Result<(), NodeError> {
        let root = Node::parent(1);
        let child = Node::insert(&root, 2)?;
        let child2 = Node::insert(&root, 4)?;
        let gc = Node::insert(&child2, 4)?;

        let p = Node::pop(&child2, &gc)?;
        assert!(child.borrow().is_leaf());
        Ok(())
    }

    #[test]
    fn upgrade_test() -> Result<(), NodeError> {
        let mut leaf = Node::leaf(42, None);

        // insert child node into leaf to make it parent.
        let child = Node::leaf(100, Some(leaf.clone()));
        Node::upgrade(&mut leaf, &child)?;
        assert!(leaf.borrow().is_root());
        Ok(())
    }

    #[test]
    fn failure_downgrade() {
        let leaf = Node::leaf(42, None);
        assert!(Node::downgrade(&leaf) == Err(NodeError::DowngradeNotParent));
    }

    #[test]
    fn failure_downgrade_root_node() {
        let root = Node::parent(42);
        assert!(Node::downgrade(&root) == Err(NodeError::RootDowngradeNotAllowed));
    }

    #[test]
    fn failure_upgrade_parent() {
        let root = Node::parent(42);
        let leaf = Node::leaf(101, Some(root.clone()));
        assert!(Node::upgrade(&root, &leaf) == Err(NodeError::ParentUpgradeNotAllowed));
    }

    #[test]
    #[cfg(feature = "std")]
    fn dispaly() -> Result<(), NodeError> {
        let root = Node::parent(42);
        let leaf = Node::insert(&root, 420)?;

        println!("{}", root.borrow());
        println!("{}", leaf.borrow());
        Ok(())
    }

    #[test]
    fn test_iter_method() -> Result<(), NodeError> {
        let root = Node::parent(1);
        let child = Node::insert(&root, 2)?;
        let _ = Node::insert(&root, 3)?;
        let _ = Node::insert(&root, 4)?;
        let _ = Node::insert(&child, 5)?;

        let mut counter = 1;
        let mut iterator = Node::iter(root);
        while let Some(item) = iterator.next() {
            assert!(item.borrow_mut().value().eq(&counter));
            counter += 1;
        }

        Ok(())
    }
}
