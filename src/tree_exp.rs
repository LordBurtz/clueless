use std::collections::HashMap;

use crate::json_models::{
    Offer
};

struct RegionNode<'a> {
    id: u32,
    left_subregion: Option<Box<RegionNode<'a>>>,
    right_subregion: Option<Box<RegionNode<'a>>>,
    nodes: &'a mut Vec<Offer>,
}



pub fn build_region_tree() -> RegionNode {
    RegionNode {
        id: 0,
        left_subregion: None,
        right_subregion: None,
        nodes: Vec::new().as_mut(),
    }
}

pub fn build_node(id: u32) -> Option<Box<RegionNode>> {
    Some(Box::new(RegionNode {
        id,
        left_subregion: None,
        right_subregion: None,
        nodes: Vec::new().as_mut(),
    }))
}

pub fn getTree() -> OctTree<Offer, OctVec> {
    OctTree::with_capacity(117, 117)
}






#[derive(Debug)]
struct TreeNode {
    id: u32,
    value: Vec<Offer>,
    children: Vec<TreeNode>, // IDs of child nodes
}

#[derive(Debug)]
struct Tree {
    nodes: HashMap<u32, TreeNode>, // Map of node ID to TreeNode
}

impl TreeNode {
    fn get_children_values(&self) -> Vec<&Offer> {
        let mut result = &self.children
                .iter()
                .map(|node| node.get_children_values())
                .collect::<Vec<&Offer>>();

        result.append(&self.value)
    }
}

impl Tree {
    // Create a new empty tree
    fn new() -> Self {
        Tree {
            nodes: HashMap::new(),
        }
    }

    fn get_node(&self, id: &u32) -> Option<&TreeNode> {
        self.nodes.get(id)
    }

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

    fn add_node(&mut self, id: u32, value: Vec<Offer>, parent_id: Option<&str>) {
        let node = TreeNode {
            id: id.to_string(),
            value: value.to_string(),
            children: Vec::new(),
        };

        self.nodes.insert(id.to_string(), node);

        if let Some(parent_id) = parent_id {
            if let Some(parent_node) = self.nodes.get_mut(parent_id) {
                parent_node.children.push(id.to_string());
            } else {
                eprintln!("Parent ID '{}' not found!", parent_id);
            }
        }
    }
}
