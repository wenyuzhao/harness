use std::collections::{LinkedList, VecDeque};

pub trait Queue<E>: IntoIterator<Item = E> + Default {
    fn push_back(&mut self, element: E);
    fn pop_front(&mut self) -> Option<E>;
}

impl<E> Queue<E> for VecDeque<E> {
    fn push_back(&mut self, element: E) {
        VecDeque::push_back(self, element);
    }

    fn pop_front(&mut self) -> Option<E> {
        VecDeque::pop_front(self)
    }
}

impl<E> Queue<E> for LinkedList<E> {
    fn push_back(&mut self, element: E) {
        LinkedList::push_back(self, element);
    }

    fn pop_front(&mut self) -> Option<E> {
        LinkedList::pop_front(self)
    }
}

#[cfg(feature = "vec_deque")]
pub type DefaultQueue<E> = VecDeque<E>;

#[cfg(feature = "linked_list")]
pub type DefaultQueue<E> = LinkedList<E>;
