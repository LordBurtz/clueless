use std::collections::{HashMap, BTreeMap, VecDeque};

use slotmap::{DenseSlotMap, SecondaryMap, SlotMap};

#[derive(Clone)]
pub struct DenseStore<'a> {
    pub all: Vec<Offer<'a>>,
}

impl DenseStore<'_> {
    pub fn new() -> Self {
        Self { all: Vec::new() }
    }

    pub fn insert(&mut self, offer: Offer) {
        self.all.push(offer);
    }
}

impl DenseStore<'_> {
    fn new() -> Self {
        DenseStore {
            map: Default::default(),
        }
    }

    fn insert(&mut self, key: usize, value: usize) {

    }
}

use crate::json_models::{
    Offer
};

#[derive(Debug)]
struct TreeNode<'a> {
    id: u32,
    value: Vec<Offer<'a>>,
    children: Vec<TreeNode<'a>>, // IDs of child nodes
}

impl TreeNode<'_> {
    fn get_children_values(&self) -> Vec<&Offer> {
        let mut result = &self.children
            .iter()
            .map(|node| node.get_children_values())
            .collect::<Vec<&Offer>>();

        result.append(&self.value)
    }
}


#[derive(Debug)]
struct Tree<'a> {
    root: TreeNode<'a>,
    nodes: HashMap<u32, TreeNode<'a>>, // Map of node ID to TreeNode
}

impl Tree<'_> {
    // Create a new empty tree
    fn new() -> Self {
        Tree {
            root: TreeNode {
                id: 0,
                value: vec![],
                children: vec![],
            },
            nodes: HashMap::new(),
        }
    }

    fn insert_offer(&mut self, offer: Offer) {
        let node = self.nodes.get(&offer.most_specific_region_ID).unwrap();
        node.value.push(offer);

    }

    fn get_node(&self, id: &u32) -> Option<&TreeNode> {
        self.nodes.get(id)
    }

    //fn get_node_anywhere(&self, id : &u32) -> Option<&TreeNode>

    fn get_children_values(&self, id: &u32) -> Vec<&Offer> {
        if let Some(node) = self.nodes.get(id) {
            node.children
                .iter()
                .filter_map(|child_id| self.nodes.get(child_id))
                .flat_map(|child_node| child_node.value)
                .collect()
        } else {
            // eprintln!("Node ID '{}' not found!", id);
            vec![]
        }
    }


    fn get_offers_for(&self, id: &u32) -> Option<Vec<&Offer>> {
        let parent = self.get_node(id).unwrap();



        None
    }

    fn add_node(&mut self, id: u32, value: Vec<Offer>, parent_id: Option<&u32>) {
        let node = TreeNode {
            id,
            value,
            children: Vec::new(),
        };

        self.nodes.insert(id, node);

        if let Some(parent_id) = parent_id {
            if let Some(parent_node) = self.nodes.get_mut(parent_id) {
                parent_node.children.push(self.nodes.get(&id).ok_or(0)?);
            } else {
                eprintln!("Parent ID '{}' not found!", parent_id);
            }
        }
    }


}
