use std::collections::HashMap;
use std::collections::hash_map;
use std::slice;

pub struct Track {
    nodes: Vec<Node>,
    name: String,
}

#[repr(C)]
pub enum CAPIInterpType {
    None = 0,
    Linear = 1
}

#[repr(C)]
pub struct CAPINodeIterator {
    track: *const Track,
    index: usize,
}

impl CAPIInterpType {
    pub fn into_func(self) -> Interpolator {
        match self {
            CAPIInterpType::None => interp_none,
            CAPIInterpType::Linear => interp_linear
        }
    }
}

impl Track {
    fn new(name: &str) -> Self {
        let mut track = Track {
            nodes: vec![],
            name: String::from(name),
        };

        track.internal_add_node(0, &Node::new(0,0_f64, interp_none));
        track
    }

    fn internal_add_node(&mut self, index: usize, node: &Node) {
        if index >= self.nodes.len() {
            self.nodes.push(*node)
        }
        else {
            self.nodes.insert(index, *node);
        }
    }

    pub fn get_name(&self) -> &str { &self.name }

    pub fn add_node(&mut self, add_node: &Node)-> Option<&'static str> {
        if add_node.get_time() == 0 { 
            return Some("Inserting a node with at_time=0 is not allowed."); 
        }

        let mut prev_time = self.nodes[0].get_time();
        let mut insert_index = None;

        for (i, node) in self.nodes.iter().enumerate().skip(1) {
            let time = node.get_time();
            if time == add_node.get_time(){ 
                return Some("A node already exists at this time point.");
            }

            if node.get_time() > prev_time && time > node.get_time() {
                insert_index = Some(i);
                break;
            }

            prev_time = time;
        }

        let index = match insert_index { Some(index) => index, None => self.nodes.len() };

        self.internal_add_node(index, add_node);
        None
    }

    pub fn get_node_at(&self, time: u32) -> Option<&Node> {
        let (_i, node) = self.internal_get_node_at(time);
        match node {
            Some(node) => Some(node),
            None => None
        }
    }

    pub fn get_value_at(&self, time: u32) -> f64 {
        let (left, right) = self.internal_get_nodes_between(time);
        let right = match right {
            Some(node) => node,
            None => return left.get_value()
        };

        let t = (time as f64 - left.get_time() as f64) / (right.get_time() as f64 - left.get_time() as f64);

        (right.interp)(left, right, t)
    }

    pub fn nodes(&mut self) -> slice::Iter<Node> { self.nodes.iter() }
    
    pub fn del_node_at(&mut self, time: u32) -> Option<&'static str> {
        match self.internal_get_node_index_at(time) {
            Some(index) => { self.nodes.remove(index); None }
            None => Some("Could not find node at the given time.")
        }
    }

    pub fn update_node_at(&mut self, time: u32, node: &Node) -> Option<&'static str> {
        match self.internal_get_node_index_at(time) {
            Some(index) => { 
                if (index + 1 == self.nodes.len()) 
                    || (self.nodes[index].get_time() == node.get_time()) {
                    self.nodes[index] = *node; 
                    return None 
                }

                self.add_node(node);
                None
            }
            None => Some("Could not find node at the given time.")
        }
    }


    fn internal_get_nodes_between(&self, time: u32) -> (&Node, Option<&Node>) {
        let mut prev_node = &self.nodes[0];

        for node in self.nodes.iter().skip(1) {
            if time >= prev_node.get_time() && node.get_time() >= time {
                return (prev_node, Some(node))
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

    pub fn get_track(&mut self, name: &str) -> &Track { 
        self.try_add_track(name);
        self.tracks.get(name).unwrap()
    }

    pub fn get_track_mut(&mut self, name: &str) -> &mut Track { 
        self.try_add_track(name);
        self.tracks.get_mut(name).unwrap()
    }

    pub fn del_track(&mut self, name: &str) -> bool {
        match self.tracks.remove(name) {
            Some(_) => true,
            None => false
        }
    }

    fn try_add_track(&mut self, name: &str) {
        if self.tracks.contains_key(name) {
            return
        }

        let track = Track::new(name);
        // TODO : we dupe the string here twice, can we get that down to one dupe?
        let result = self.tracks.insert(String::from(name), track); 
        
        assert_eq!(result.is_none(), true, "key: {}", name);
    }


    pub fn tracks(&mut self) -> TimelineTrackIter { TimelineTrackIter { iter: self.tracks.iter() }}
}

pub type Interpolator = fn(from: &Node, to: &Node, t: f64) -> f64;

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

pub mod ffi {

    use super::*;
    use std::boxed::Box;
    use std::ffi::CStr;
    use std::os::raw::*;
    use std::ptr;


    #[no_mangle]
    pub unsafe extern "C" fn demy_tl_new() -> *mut Timeline {
        let tl = Box::new(Timeline::new());
        Box::into_raw(tl)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tl_free(tl: *mut Timeline) {
        if tl.is_null() { return }
        Box::from_raw(tl);
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tl_track_get(tl: *mut Timeline, name: *const c_char) -> *const Track {
        let name = CStr::from_ptr(name).to_str().unwrap();
        (*tl).get_track(name)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tl_track_del(tl: *mut Timeline, name: *const c_char) -> bool {
        let name = CStr::from_ptr(name).to_str().unwrap();
        (*tl).del_track(name)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_add_node(tr: *mut Track, time: c_uint, value: c_double, interp: CAPIInterpType) -> bool {
        let node = Node::new(time, value, interp.into_func());
        match (*tr).add_node(&node) {
            Some(_err) => false, // TODO : expose error string to C
            None => true
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_del_node(tr: *mut Track, time: c_uint) -> bool {
        match (*tr).del_node_at(time) {
            Some(_err) => false, // TODO : expose error string to C
            None => true
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_get_node(tr: *mut Track, time: c_uint) -> *const Node {
        match (*tr).get_node_at(time) {
            Some(node) => node,
            None => ptr::null()
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_iter_start(tr: *mut Track) -> *mut CAPINodeIterator {
        let data = Box::new(CAPINodeIterator { 
            track: tr as *const Track, 
            index: 0,
        });

        Box::into_raw(data)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_iter_next(ptr_to_iter: *mut *mut CAPINodeIterator) {
        if ptr_to_iter.is_null() { return }
        let iter = *ptr_to_iter;

        if (*iter).index >= (*(*iter).track).nodes.len() {
            (*ptr_to_iter) = ptr::null_mut()
        }
        
        (*iter).index += 1
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_iter_get(iter: *mut CAPINodeIterator) -> *const Node {
        &(*(*iter).track).nodes[(*iter).index]
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_update_at(tr: *mut Track, time: c_uint, node: *const Node) -> bool{
        match (*tr).update_node_at(time, &*node) {
            Some(_err) => false, // TODO : expose errors to C
            None => true
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_clone(node: *const Node) -> *const Node {
        let new_node = Box::new((*node).clone());
        Box::into_raw(new_node)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_new(time: c_uint, value: c_double, interp: CAPIInterpType) {
        let new_node = Box::new(Node::new(time, value, interp.into_func()));
        Box::into_raw(new_node);
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_free(node: *mut Node) {
        if node.is_null() { return }
        Box::from_raw(node);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_node() {
        let mut tl = Timeline::new();
        let track = tl.get_track_mut("camera");

        track.add_node(&Node::new(10, 1_f64, interp_none));

        track.update_node_at(10, &Node::new(20, 2_f64, interp_none));
        assert!(track.nodes().count() == 2);

        assert!(track.get_node_at(20).unwrap().get_time() == 20);
        assert!(track.get_node_at(20).unwrap().get_value() == 2_f64);

        assert!(track.get_node_at(10).is_none());

        track.add_node(&Node::new(10, 1_f64, interp_none));
        assert!(track.nodes().count() == 3);

        assert!(track.get_node_at(20).unwrap().get_time() == 20);
        assert!(track.get_node_at(20).unwrap().get_value() == 2_f64);

        assert!(track.get_node_at(10).unwrap().get_time() == 10);
        assert!(track.get_node_at(10).unwrap().get_value() == 1_f64);

        track.update_node_at(10, &Node::new(30, 3_f64, interp_none));

        assert!(track.get_node_at(10).is_none());

        assert!(track.get_node_at(20).unwrap().get_time() == 20);
        assert!(track.get_node_at(20).unwrap().get_value() == 2_f64);

        assert!(track.get_node_at(30).unwrap().get_time() == 30);
        assert!(track.get_node_at(30).unwrap().get_value() == 3_f64);
    }

    #[test]
    fn track_deletion() {
        let mut tl = Timeline::new();
        assert!(tl.tracks().count() == 0);

        tl.get_track("camera");
        assert!(tl.tracks().count() == 1);

        tl.del_track("not_the_camera");
        assert!(tl.tracks().count() == 1);

        tl.del_track("camera");
        assert!(tl.tracks().count() == 0);
    }

    #[test]
    fn interpolation() {
        let mut tl = Timeline::new();
        {
            let mut track = tl.get_track_mut("camera");
            assert!(track.add_node(&Node::new(10, 1_f64, interp_linear)).is_none());
            assert!(track.add_node(&Node::new(20, 2_f64, interp_linear)).is_none());

            assert!(track.nodes().len() == 3);
        }

        let track = tl.get_track("camera");

        let val = track.get_value_at(5);
        assert!(0.001 > (0.5_f64 - val).abs(), "val: {}", val);
        
        let val = track.get_value_at(15);
        assert!(0.001 > (1.5_f64 - val).abs(), "val: {}", val);
    }

    #[test]
    fn no_duplicate_tracks() {
        let name = "camera";
        let mut tl = Timeline::new();

        tl.get_track(name);
        tl.get_track(name);

        assert_eq!(tl.tracks().count(), 1);
    }

    #[test]
    fn default_zero_node() {
        let mut tl = Timeline::new();
        let mut track = tl.get_track_mut("camera");

        assert_eq!(track.nodes().next().unwrap().get_time(), 0);
    }

    #[test]
    fn no_duplicate_nodes() {
        let mut tl = Timeline::new();
        let mut track = tl.get_track_mut("camera");
        let time = 1;

        assert!(track.add_node(&Node::new(1, 0_f64, interp_none)).is_none());
        assert!(track.add_node(&Node::new(1, 0_f64, interp_none)).is_some());

        assert_eq!(track.nodes().count(), 2); // implcit 0
    }

    #[test]
    fn timeline_mutation() {
        let mut tl = Timeline::new();

        {
            let mut track = tl.get_track_mut("camera.x");
            track.add_node(&Node::new(10, 1_f64, interp_linear));
        }

        for i in 0..50 {
            tl.get_track(&i.to_string());
        }

        let track = tl.get_track("camera.x");
        let node = track.get_node_at(10).unwrap();
        assert_eq!(node.get_time(), 10);
        assert_eq!(node.get_value(), 1_f64);
    }
}