use std::collections::HashMap;
use std::collections::hash_map;
use std::slice;

pub struct TrackNodeIterator<'node> {
    iter: slice::IterMut<'node, Node>,
}

impl<'node> Iterator for TrackNodeIterator<'node> {
    type Item = &'node mut Node;

    fn next(&mut self) -> Option<&'node mut Node> { self.iter.next() }
}

pub struct Track {
    nodes: Vec<Node>,
    name: String,
}

impl Track {
    fn new(name: &str) -> Self {
        let mut track = Track {
            nodes: vec![],
            name: String::from(name),
        };

        track.internal_add_node(0, Node::new(0,0_f64, interp_none));
        track
    }

    fn internal_add_node(&mut self, index: usize, node: Node) {
        self.nodes.insert(index, node);
    }

    pub fn get_name(&self) -> &str { &self.name }

    pub fn add_node(&mut self, at_time: u32, value: f64, interp: Interpolator) -> Option<&'static str> {
        if at_time == 0 { 
            return Some("Inserting a node with at_time=0 is not allowed.."); 
        }

        let mut prev_time = self.nodes[0].get_time();
        let mut insert_index = None;

        for (i, node) in self.nodes.iter().enumerate().skip(1) {
            let time = node.get_time();
            if time == at_time { 
                return Some("A node already exists at this time point.");
            }

            if at_time > prev_time && time > at_time {
                insert_index = Some(i);
                break;
            }

            prev_time = time;
        }

        let index = match insert_index { Some(index) => index, None => self.nodes.len() };

        self.internal_add_node(index, Node::new(at_time, value, interp));
        None
    }

    fn internal_get_nodes_between(&self, time: u32) -> (&Node, Option<&Node>) {
        let mut prev_node = &self.nodes[0];

        for node in self.nodes.iter().skip(1) {
            if time >= prev_node.get_time() && node.get_time() >= time {
                return (node, Some(prev_node));
            }

            prev_node = node;
        }

        (prev_node, None)
    }

    fn internal_get_node_at(&self, time: u32) -> (usize, Option<&Node>) {
        for (i, node) in self.nodes.iter().enumerate() {
            if node.get_time() == time {
                return (i, Some(node))
            }
        }

        (0, None)
    }

    fn internal_get_node_index_at(&self, time: u32) -> Option<usize> {
        let (index, opt_node) = self.internal_get_node_at(time);
        match opt_node {
            Some(_node) => Some(index),
            None => None
        }
    }

    pub fn view_node_at(&self, time: u32) -> Option<Node> {
        let (_i, node) = self.internal_get_node_at(time);
        match node {
            Some(node) => Some(*node),
            None => None
        }
    }

    pub fn view_nodes_between(&self, time: u32) -> (Node, Option<Node>) {
        let (l, r) = self.internal_get_nodes_between(time);

        match r {
            Some(r) => (*l, Some(*r)),
            None => (*l, None)
        }
    }

    pub fn view_value(&self, time: u32) -> f64 {
        let (left, right) = self.internal_get_nodes_between(time);
        let right = match right {
            Some(node) => node,
            None => return left.get_value()
        };

        let t = (time - left.get_time()) as f64 / (right.get_time() - left.get_time()) as f64;

        (left.interp)(left, right, t)
    }

    pub fn view_nodes(&self) -> Vec<Node> { self.nodes.clone() }
    
    pub fn destroy_node_at(&mut self, time: u32) -> Option<&'static str> {

        match self.internal_get_node_index_at(time) {
            Some(index) => { self.nodes.remove(index); None }
            None => Some("Could not find node at the given time.")
        }
    }

    pub fn update_node_at(&mut self, time: u32, node: &Node) -> Option<&'static str> {

        match self.internal_get_node_index_at(time) {
            Some(index) => { self.nodes[index] = *node; None }
            None => Some("Could not find node at the given time.")
        }
    }

    pub fn view_next_node(&self, node: &Node) -> Result<Option<Node>, &'static str> {
        match self.internal_get_node_index_at(node.get_time()) {
            Some(index) => {
                if index >= self.nodes.len() {
                    Ok(None)
                }
                else {
                    Ok(Some(self.nodes[index + 1]))
                }
            }
            None => Err("Could not find node at the given time.")
        }
    }

    pub fn view_previous_node(&self, node: &Node) -> Result<Option<Node>, &'static str> {
        match self.internal_get_node_index_at(node.get_time()) {
            Some(index) => {
                if 0 >= index {
                    Ok(None)
                }
                else {
                    Ok(Some(self.nodes[index - 1]))
                }
            }
            None => Err("Could not find node at the given time.")
        }
    }

    pub fn nodes<'track>(&'track mut self) -> TrackNodeIterator<'track> { 
        TrackNodeIterator { iter: self.nodes.iter_mut() } 
    }
}

pub struct Timeline {
    tracks: HashMap<String, Track>, 
}

pub struct TimelineTrackIter<'timeline> {
    iter: hash_map::Iter<'timeline, String, Track>,
}
impl<'timeline >Iterator for TimelineTrackIter<'timeline> {
    type Item = &'timeline Track;

    fn next(&mut self) -> Option<&'timeline Track> {
        match self.iter.next() {
            Some((_k ,v)) => Some(v),
            None => None
        }
    }
}

impl Timeline {
    pub fn new() -> Self {
        Timeline {
            tracks: HashMap::new()
        }
    }

    pub fn get_track(&self, name: &str) -> Option<&Track> { self.tracks.get(name) }
    pub fn get_track_mut(&mut self, name: &str) -> Option<&mut Track> { self.tracks.get_mut(name) }

    pub fn new_track(&mut self, name: &str) -> Option<&mut Track> {
        if self.tracks.contains_key(name) { 
            return None;
        }

        let track = Track::new(name);
        // TODO : we dupe the string here twice, can we get that down to one dupe?
        let result = self.tracks.insert(String::from(name), track); 
        
        assert_eq!(result.is_none(), true, "key: {}", name);
        
        self.tracks.get_mut(name)
    }

    pub fn tracks(&mut self) -> TimelineTrackIter { TimelineTrackIter { iter: self.tracks.iter() }}
}

type Interpolator = fn(from: &Node, to: &Node, t: f64) -> f64;

pub fn interp_none(from: &Node, _to: &Node, _t: f64) -> f64 { from.get_value() }
pub fn interp_linear(from: &Node, to: &Node, t: f64) -> f64 {
    from.get_value() * (1_f64 - t) + (t * to.get_value())
}

#[derive(Copy, Clone)]
pub struct Node {
    time: u32,
    value: f64,
    interp: Interpolator,
}

impl Node {
    pub fn new(time: u32, value: f64, interp: Interpolator) -> Self {
        Node { time, value, interp }
    }

    pub fn get_time(&self) -> u32 { self.time }
    pub fn set_time(&mut self, time: u32) { self.time = time }

    pub fn get_value(&self) -> f64 { self.value }
    pub fn set_value(&mut self, value: f64) { self.value = value }

    pub fn get_interpolator(&self) -> Interpolator { self.interp }
    pub fn set_interpolator(&mut self, interp: Interpolator) { self.interp = interp }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_duplicate_tracks() {
        let name = "camera";
        let mut tl = Timeline::new();

        assert!(tl.new_track(name).is_some());
        assert!(tl.new_track(name).is_none());

        assert_eq!(tl.tracks().count(), 1);
    }

    #[test]
    fn default_zero_node() {
        let mut tl = Timeline::new();
        let mut track = tl.new_track("camera").unwrap();

        assert_eq!(track.nodes().next().unwrap().get_time(), 0);
    }

    #[test]
    fn no_duplicate_nodes() {
        let mut tl = Timeline::new();
        let mut track = tl.new_track("camera").unwrap();
        let time = 1;

        assert!(track.add_node(1, 0_f64, interp_none).is_none());
        assert!(track.add_node(1, 0_f64, interp_none).is_some());

        assert_eq!(track.nodes().count(), 2); // implcit 0
    }

    #[test]
    fn timeline_mutation() {
        let mut tl = Timeline::new();

        {
            let mut track = tl.new_track("camera.x").unwrap();
            track.add_node(10, 1_f64, interp_linear);
        }

        for i in 0..50 {
            tl.new_track(&i.to_string());
        }

        let track = tl.get_track("camera.x").unwrap();
        let node = track.view_node_at(10).unwrap();
        assert_eq!(node.get_time(), 10);
        assert_eq!(node.get_value(), 1_f64);
    }
}
