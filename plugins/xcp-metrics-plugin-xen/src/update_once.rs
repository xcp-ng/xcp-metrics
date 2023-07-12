use std::cell::{Cell, Ref, RefCell};

pub trait Updatable {
    fn update(&mut self);
}

pub struct UpdateOnce<T: Updatable> {
    inner: RefCell<T>,
    latest_update: Cell<Option<uuid::Uuid>>,
}

impl<T: Updatable> UpdateOnce<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: RefCell::new(inner),
            latest_update: Cell::new(None),
        }
    }

    pub fn update(&self, token: uuid::Uuid) {
        if self.latest_update.get() != Some(token) {
            // Update value
            self.inner.borrow_mut().update();

            self.latest_update.replace(token.into());
        }
    }

    pub fn borrow(&self) -> Ref<T> {
        self.inner.borrow()
    }
}
