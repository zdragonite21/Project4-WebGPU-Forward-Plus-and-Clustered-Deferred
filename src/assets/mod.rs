use std::marker::PhantomData;

pub mod gltf_importer;
pub mod gltf_loader;
pub mod material;
pub mod mesh;
pub mod texture;

pub struct Handle<T> {
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl<T> Eq for Handle<T> {}

pub struct Assets<T> {
    items: Vec<T>,
}

impl<T> Assets<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    pub fn add(&mut self, asset: T) -> Handle<T> {
        self.items.push(asset);

        Handle {
            index: self.items.len() - 1,
            _marker: PhantomData,
        }
    }
    pub fn get(&self, handle: Handle<T>) -> &T {
        &self.items[handle.index]
    }
}
