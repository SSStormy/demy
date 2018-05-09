use std::collections::HashMap;
use std::collections::hash_map;
use std::slice;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

#[derive(Serialize, Deserialize)]
pub struct Track {
    nodes: Vec<Node>,
    name: String,
}

#[repr(C)]
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum InterpType {
    None = 0,
    Linear = 1
}

#[repr(C)]
pub struct CAPINodeIterator {
    track: *const Track,
    index: usize,
}

impl InterpType {
    pub fn to_func(&self) -> Interpolator {
        match self {
            &InterpType::None => interp_none,
            &InterpType::Linear => interp_linear
        }
    }
}

impl Track {
    fn new(name: &str) -> Self {
        let mut track = Track {
            nodes: vec![],
            name: String::from(name),
        };

        track.internal_add_node(0, &Node::new(0,0_f64, InterpType::None));
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

        for (i, cur_node) in self.nodes.iter().enumerate().skip(1) {
            let cur_time = cur_node.get_time();
            let add_time = add_node.get_time();

            if cur_time == add_time { 
                return Some("A node already exists at this time point.");
            }

            if cur_time > add_time && add_time > prev_time {
                insert_index = Some(i);
                break;
            }

            prev_time = cur_time;
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

        (right.interp.to_func())(left, right, t)
    }

    pub fn nodes(&self) -> slice::Iter<Node> { self.nodes.iter() }
    
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

                self.del_node_at(time);
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

#[derive(Serialize, Deserialize)]
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

    pub fn save(&self) -> Result<String, &'static str> {
        match serde_json::to_string(self) {
            Ok(result) => Ok(result),
            Err(_err) => Err("Failed to save timeline.")
        }
    }

    pub fn load(buffer: &str) -> Result<Timeline, &'static str> {
        match serde_json::from_str(&buffer) {
            Ok(val) => Ok(val),
            Err(_err) => Err("Failed to load timeline.")
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

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Node {
    time: u32,
    value: f64,
    interp: InterpType,
}

impl Node {
    pub fn new(time: u32, value: f64, interp: InterpType) -> Self {
        Node { time, value, interp }
    }

    pub fn get_time(&self) -> u32 { self.time }
    pub fn set_time(&mut self, time: u32) { self.time = time }

    pub fn get_value(&self) -> f64 { self.value }
    pub fn set_value(&mut self, value: f64) { self.value = value }

    pub fn get_interpolator(&self) -> InterpType{ self.interp }
    pub fn set_interpolator(&mut self, interp: InterpType) { self.interp = interp }
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
    pub unsafe extern "C" fn demy_tl_track_get(tl: *mut Timeline, name: *const c_char) -> *mut Track {
        let name = CStr::from_ptr(name).to_str().unwrap();
        (*tl).get_track_mut(name)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tl_track_del(tl: *mut Timeline, name: *const c_char) -> bool {
        let name = CStr::from_ptr(name).to_str().unwrap();
        (*tl).del_track(name)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_add_node(tr: *mut Track, time: c_uint, value: c_double, interp: InterpType) -> bool {
        let node = Node::new(time, value, interp);
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
    pub unsafe extern "C" fn demy_tr_get_node(tr: *const Track, time: c_uint) -> *const Node {
        match (*tr).get_node_at(time) {
            Some(node) => node,
            None => ptr::null()
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_iter_begin(tr: *const Track) -> *mut CAPINodeIterator {
        let data = Box::new(CAPINodeIterator { 
            track: tr as *const Track, 
            index: 0,
        });

        Box::into_raw(data)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_iter_end(tr: *const Track) -> *mut CAPINodeIterator {
        let data = Box::new(CAPINodeIterator { 
            track: tr as *const Track, 
            index: (*tr).nodes.len(),
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
    pub unsafe extern "C" fn demy_tr_iter_free(iter: *mut CAPINodeIterator) {
        if iter.is_null() { return }
        Box::from_raw(iter);
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_iter_are_eq(a: *const CAPINodeIterator, b: *const CAPINodeIterator) -> bool {
        if a.is_null() || b.is_null() { return false; }
        return (*a).index == (*b).index;
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tr_iter_get(iter: *const CAPINodeIterator) -> *const Node {
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
    pub unsafe extern "C" fn demy_node_clone(node: *const Node) -> *mut Node {
        let new_node = Box::new((*node).clone());
        Box::into_raw(new_node)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_new(time: c_uint, value: c_double, interp: InterpType) -> *mut Node{
        let new_node = Box::new(Node::new(time, value, interp));
        Box::into_raw(new_node)
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_free(node: *mut Node) {
        if node.is_null() { return }
        Box::from_raw(node);
    }

    use std::fs;
    use std::io::Read;
    use std::io::Write;
    use std::error::Error;

    #[no_mangle]
    pub unsafe extern "C" fn demy_tl_save(tl: *const Timeline, path: *const c_char) -> bool {
        if tl.is_null() { return false; }

        let path = match CStr::from_ptr(path).to_str() {
            Ok(path) => path,
            Err(e) => { 
                println!("{}", e.description());
                return false;
            }
        };

        let mut fd = match fs::File::create(path) {
            Ok(fd) => fd,
            Err(e) => {
                println!("{}", e.description());
                return false;
            }
        };

        let data = match (*tl).save() {
            Ok(data) => data,
            Err(e) => {
                println!("{}", e);
                return false;
            }
        };

        match fd.write_all(&data.into_bytes()) {
            Ok(_result) => true,
            Err(e) => {
                println!("{}", e.description());
                false
            }
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_tl_load(path: *const c_char) -> *mut Timeline {
        if path.is_null() { return ptr::null_mut(); }

        let path_cstr = CStr::from_ptr(path).to_str();
        let path = match  path_cstr {
            Ok(p) => p,
            Err(e) =>  {
                println!("{}", e.description());
                return ptr::null_mut();
            }
        };

        let mut fd = match fs::File::open(path) {
            Ok(fd) => fd,
            Err(e) => { 
                println!("{}", e.description());
                return ptr::null_mut()
            }
        };

        let mut contents = String::new();
        match fd.read_to_string(&mut contents) {
            Ok (_num) => (),
            Err(e) => { 
                println!("{}", e.description());
                return ptr::null_mut()
            }
        };

        match Timeline::load(&contents) {
            Ok(tl) => Box::into_raw(Box::new(tl)),
            Err(e) => { 
                println!("{}", e);
                return ptr::null_mut()
            }
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_set_interp(node: *mut Node, interp: InterpType) {
        if node.is_null() { return }
        (*node).interp = interp;
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_get_interp(node: *mut Node) -> InterpType {
        if node.is_null() { return InterpType::None }
        (*node).interp
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_set_value(node: *mut Node, value: c_double) {
        if node.is_null() { return }
        (*node).value = value;
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_get_value(node: *mut Node) -> c_double {
        if node.is_null() { return 0_f64 }
        (*node).value
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_set_time(node: *mut Node, time: c_uint) {
        if node.is_null() { return }
        (*node).time = time;
    }

    #[no_mangle]
    pub unsafe extern "C" fn demy_node_get_time(node: *mut Node) -> c_uint {
        if node.is_null() { return 0 }
        (*node).time
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_node() {
        let mut tl = Timeline::new();
        let track = tl.get_track_mut("camera");

        track.add_node(&Node::new(10, 1_f64, InterpType::None));

        track.update_node_at(10, &Node::new(20, 2_f64, InterpType::None));
        assert!(track.nodes().count() == 2);

        assert!(track.get_node_at(20).unwrap().get_time() == 20);
        assert!(track.get_node_at(20).unwrap().get_value() == 2_f64);

        assert!(track.get_node_at(10).is_none());

        track.add_node(&Node::new(10, 1_f64, InterpType::None));
        assert!(track.nodes().count() == 3);

        assert!(track.get_node_at(20).unwrap().get_time() == 20);
        assert!(track.get_node_at(20).unwrap().get_value() == 2_f64);

        assert!(track.get_node_at(10).unwrap().get_time() == 10);
        assert!(track.get_node_at(10).unwrap().get_value() == 1_f64);

        track.update_node_at(10, &Node::new(30, 3_f64, InterpType::None));

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
            let track = tl.get_track_mut("camera");
            assert!(track.add_node(&Node::new(10, 1_f64, InterpType::Linear)).is_none());
            assert!(track.add_node(&Node::new(20, 2_f64, InterpType::Linear)).is_none());

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
        let track = tl.get_track_mut("camera");

        assert_eq!(track.nodes().next().unwrap().get_time(), 0);
    }

    #[test]
    fn no_duplicate_nodes() {
        let mut tl = Timeline::new();
        let track = tl.get_track_mut("camera");

        assert!(track.add_node(&Node::new(1, 0_f64, InterpType::None)).is_none());
        assert!(track.add_node(&Node::new(1, 0_f64, InterpType::None)).is_some());

        assert_eq!(track.nodes().count(), 2); // implcit 0
    }

    #[test]
    fn timeline_mutation() {
        let mut tl = Timeline::new();

        {
            let track = tl.get_track_mut("camera.x");
            track.add_node(&Node::new(10, 1_f64, InterpType::Linear));
        }

        for i in 0..50 {
            tl.get_track(&i.to_string());
        }

        let track = tl.get_track("camera.x");
        let node = track.get_node_at(10).unwrap();
        assert_eq!(node.get_time(), 10);
        assert_eq!(node.get_value(), 1_f64);
    }

    #[test]
    fn node_vec_is_ordered() {
        let mut tl = Timeline::new();
        let track = tl.get_track_mut("camera.x");

        track.add_node(&Node::new(5, 5_f64, InterpType::Linear));
        track.add_node(&Node::new(2, 2_f64, InterpType::Linear));

        let mut prev_node = track.nodes().next().unwrap();
        for node in track.nodes().skip(1) {
            assert!(prev_node.get_time() < node.get_time());
            prev_node = node;
        }
    }

    #[test]
    fn serialize_deserialize() {

        let serialized: String;
        let track1 = "camera.x";
        let track2 = "camera.y";

        {
            let mut tl = Timeline::new();
            let t1_node1 = Node::new(10, 1_f64, InterpType::Linear);
            let t1_node2 = Node::new(20, 2_f64, InterpType::Linear);

            let t2_node1 = Node::new(10, 4_f64, InterpType::Linear);
            let t2_node2 = Node::new(20, 8_f64, InterpType::Linear);

            {
                let track = tl.get_track_mut(track1);
                track.add_node(&t1_node1);
                track.add_node(&t1_node2);
            }

            {
                let track = tl.get_track_mut(track2);
                track.add_node(&t2_node1);
                track.add_node(&t2_node2);
            }

            serialized = tl.save().unwrap();
        }

        {
            let mut tl = Timeline::load(&serialized).unwrap();

            assert_eq!(tl.tracks().count(), 2);

            {
                let track = tl.get_track(track1);
                assert_eq!(track.nodes().count(), 3);
                let val = track.get_value_at(5);
                assert!(0.001 > (0.5_f64 - val).abs(), "val: {}", val);
                let val = track.get_value_at(15);
                assert!(0.001 > (1.5_f64 - val).abs(), "val: {}", val);
            }

            {
                let track = tl.get_track(track2);
                assert_eq!(track.nodes().count(), 3);
                let val = track.get_value_at(5);
                assert!(0.001 > (2_f64 - val).abs(), "val: {}", val);
                let val = track.get_value_at(15);
                assert!(0.001 > (6_f64 - val).abs(), "val: {}", val);
            }
        }
    }
}
