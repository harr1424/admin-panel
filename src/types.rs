use std::{collections::HashSet, sync::{Arc, Mutex}};

#[derive(Clone)]
pub struct InstructorRepo(pub Arc<Mutex<HashSet<String>>>);

#[derive(Clone)]
pub struct HostRepo(pub Arc<Mutex<HashSet<String>>>);

impl InstructorRepo {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }

    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashSet<String>>, std::sync::PoisonError<std::sync::MutexGuard<'_, HashSet<String>>>> {
        self.0.lock()
    }
}

impl HostRepo {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }

    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashSet<String>>, std::sync::PoisonError<std::sync::MutexGuard<'_, HashSet<String>>>> {
        self.0.lock()
    }
}