//! Answers extensions give for drawing the transcript, which a frame never
//! waits for. pi calls such hooks synchronously; yapi's extensions run on
//! their own threads. A frame draws without an answer it lacks and notes the
//! question, the app asks the extensions, and the next frame draws with the
//! answer.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, Mutex, PoisonError};

/// How many answers are kept; the oldest go first.
const KEPT: usize = 512;

/// Answers by question, shared by the frames that read them and the app
/// that fills them in.
pub struct Deferred<K, V>(Arc<Mutex<Answers<K, V>>>);

struct Answers<K, V> {
    /// `None` while a question waits for its answer.
    answers: HashMap<K, Option<V>>,
    order: VecDeque<K>,
    questions: Vec<K>,
}

impl<K, V> Clone for Deferred<K, V> {
    fn clone(&self) -> Self {
        Deferred(self.0.clone())
    }
}

impl<K: Clone + Eq + Hash, V: Clone> Deferred<K, V> {
    pub fn new() -> Self {
        Deferred(Arc::new(Mutex::new(Answers {
            answers: HashMap::new(),
            order: VecDeque::new(),
            questions: Vec::new(),
        })))
    }

    /// The answer to `key`, or `None` while there is none, when the
    /// question is noted for [`Deferred::questions`] once.
    pub fn get(&self, key: &K) -> Option<V> {
        let mut answers = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(answer) = answers.answers.get(key) {
            return answer.clone();
        }
        answers.questions.push(key.clone());
        answers.keep(key.clone(), None);
        None
    }

    /// The questions noted since the last call.
    pub fn questions(&self) -> Vec<K> {
        std::mem::take(
            &mut self
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .questions,
        )
    }

    /// Stores the answer to `key`.
    pub fn answer(&self, key: K, value: V) {
        let mut answers = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match answers.answers.get_mut(&key) {
            Some(answer) => *answer = Some(value),
            None => answers.keep(key, Some(value)),
        }
    }

    /// Forgets every answer and question, as when the extensions change.
    pub fn clear(&self) {
        let mut answers = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        answers.answers.clear();
        answers.order.clear();
        answers.questions.clear();
    }
}

impl<K: Clone + Eq + Hash, V> Answers<K, V> {
    fn keep(&mut self, key: K, value: Option<V>) {
        self.answers.insert(key.clone(), value);
        self.order.push_back(key);
        while self.order.len() > KEPT {
            if let Some(oldest) = self.order.pop_front() {
                self.answers.remove(&oldest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_once_and_keeps_answers() {
        let deferred: Deferred<u32, &str> = Deferred::new();
        assert_eq!(deferred.get(&1), None);
        assert_eq!(deferred.get(&1), None);
        assert_eq!(deferred.questions(), [1]);
        assert!(deferred.questions().is_empty());
        deferred.answer(1, "one");
        assert_eq!(deferred.get(&1), Some("one"));
        for key in 2..=KEPT as u32 + 1 {
            deferred.answer(key, "many");
        }
        assert_eq!(deferred.get(&1), None, "the oldest answer went");
        deferred.clear();
        assert!(deferred.questions().is_empty());
    }
}
