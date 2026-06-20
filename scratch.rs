use std::cell::RefCell;
thread_local! {
    pub static FETCH_RESULT: RefCell<Option<Vec<u8>>> = RefCell::new(None);
}
