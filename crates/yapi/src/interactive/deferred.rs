//! Answers extensions give for drawing a transcript item, which a frame
//! never waits for. pi calls such hooks synchronously; yapi's extensions run
//! on their own threads. A render reads what the item's questions got so
//! far and notes the questions it still has, the app asks them after the
//! frame, and an answer redraws the item.
//!
//! Questions are kept by the order the item's render reads them, one slot
//! each, so a code block keeps its slot as it streams. A slot keeps only
//! the answer to its latest question, has at most one question out at a
//! time, and asks only the newest of those that came in meanwhile.

use std::sync::{Arc, Mutex, PoisonError};

use futures_util::future::BoxFuture;
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};
use serde_json::json;
use yapi_core::extensions::Extension;
use yapi_tui::lines::StyledLine;
use yapi_tui::markdown::MarkdownTheme;

/// A question for the extensions.
pub trait Question: Clone + PartialEq + Send + 'static {
    /// Whether the answer to `earlier`, the slot's previous question, may
    /// stand in for this one's while it waits.
    fn continues(&self, earlier: &Self) -> bool;
}

/// One transcript item's answers of one kind, shared by its renders and
/// the tasks that ask.
pub struct Deferred<Q, A>(Arc<Mutex<Slots<Q, A>>>);

struct Slots<Q, A> {
    slots: Vec<Slot<Q, A>>,
    /// How many slots the render in progress has read.
    read: usize,
}

struct Slot<Q, A> {
    /// The latest question answered, and its answer; `None` draws as if
    /// nobody answered.
    answer: Option<(Q, Option<A>)>,
    /// The question out.
    asking: Option<Q>,
    /// The newest question read and not yet asked.
    wanted: Option<Q>,
}

impl<Q, A> Clone for Deferred<Q, A> {
    fn clone(&self) -> Self {
        Deferred(self.0.clone())
    }
}

impl<Q: Question, A: Clone> Default for Deferred<Q, A> {
    fn default() -> Self {
        Deferred(Arc::new(Mutex::new(Slots {
            slots: Vec::new(),
            read: 0,
        })))
    }
}

impl<Q: Question, A: Clone> Deferred<Q, A> {
    fn lock(&self) -> std::sync::MutexGuard<'_, Slots<Q, A>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether `other` is this item's.
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Starts a render of the item.
    pub fn begin(&self) {
        self.lock().read = 0;
    }

    /// Ends the render, forgetting the slots it did not read. Returns
    /// whether questions wait to be asked.
    pub fn end(&self) -> bool {
        let mut slots = self.lock();
        let read = slots.read;
        slots.slots.truncate(read);
        slots
            .slots
            .iter()
            .any(|slot| slot.asking.is_none() && slot.wanted.is_some())
    }

    /// What `draw` makes of the answer to `question` and the question it
    /// answers. While `question` waits, that is the slot's previous answer
    /// when `question` continues its question. `None` draws without an
    /// answer.
    pub fn get<R>(&self, question: &Q, draw: impl FnOnce(&Q, &A) -> R) -> Option<R> {
        let mut slots = self.lock();
        let index = slots.read;
        slots.read += 1;
        if slots.slots.len() <= index {
            slots.slots.push(Slot {
                answer: None,
                asking: None,
                wanted: None,
            });
        }
        let slot = &mut slots.slots[index];
        let answered = slot
            .answer
            .as_ref()
            .is_some_and(|(asked, _)| asked == question);
        if answered || slot.asking.as_ref() == Some(question) {
            slot.wanted = None;
        } else {
            slot.wanted = Some(question.clone());
        }
        let (earlier, answer) = slot.answer.as_ref()?;
        if !answered && !question.continues(earlier) {
            return None;
        }
        answer.as_ref().map(|answer| draw(earlier, answer))
    }

    /// Takes the questions to ask now, with their slots: each slot's newest
    /// while it has none out.
    pub fn questions(&self) -> Vec<(usize, Q)> {
        let mut slots = self.lock();
        let mut out = Vec::new();
        for (index, slot) in slots.slots.iter_mut().enumerate() {
            if slot.asking.is_none()
                && let Some(question) = slot.wanted.take()
            {
                slot.asking = Some(question.clone());
                out.push((index, question));
            }
        }
        out
    }

    /// Stores the answer to the question `slot` has out. Returns whether
    /// the item may draw differently: not when the slot has moved on, nor
    /// when neither this answer nor the one before it draws anything.
    pub fn answer(&self, slot: usize, question: Q, answer: Option<A>) -> bool {
        let mut slots = self.lock();
        let Some(slot) = slots.slots.get_mut(slot) else {
            return false;
        };
        if slot.asking.as_ref() != Some(&question) {
            return false;
        }
        slot.asking = None;
        if slot.wanted.as_ref() == Some(&question) {
            slot.wanted = None;
        }
        let drew = slot
            .answer
            .as_ref()
            .is_some_and(|(_, answer)| answer.is_some());
        let draws = answer.is_some();
        slot.answer = Some((question, answer));
        drew || draws
    }
}

/// A code block's code, language and width, for the extensions' code
/// renderers.
#[derive(Clone, PartialEq)]
pub(super) struct CodeQuestion {
    code: String,
    lang: Option<String>,
    width: usize,
}

impl Question for CodeQuestion {
    /// A block that grows, as one does while it streams, keeps its colors
    /// meanwhile.
    fn continues(&self, earlier: &Self) -> bool {
        self.lang == earlier.lang && self.code.starts_with(&earlier.code)
    }
}

/// What the extensions drew for one transcript item.
#[derive(Clone, Default)]
pub(super) struct ItemAnswers {
    code: Deferred<CodeQuestion, Vec<String>>,
}

impl ItemAnswers {
    /// Starts a render of the item.
    pub fn begin(&self) {
        self.code.begin();
    }

    /// Ends the render. Returns whether questions wait to be asked.
    pub fn end(&self) -> bool {
        self.code.end()
    }
}

/// The extensions that draw parts of the transcript, in load order, as of
/// the last time the session's extensions started.
#[derive(Clone, Default)]
pub(super) struct Drawers {
    pub code: Vec<Arc<dyn Extension>>,
}

impl Drawers {
    /// Whether any extension draws part of the transcript.
    pub fn any(&self) -> bool {
        !self.code.is_empty()
    }
}

/// Which of an item's answers a question goes to.
type Kind<Q, A> = fn(&ItemAnswers) -> &Deferred<Q, A>;

fn code_answers(answers: &ItemAnswers) -> &Deferred<CodeQuestion, Vec<String>> {
    &answers.code
}

/// An extension's answer, and the errors of those asked before it, by
/// extension path.
type Answered<A> = (Option<A>, Vec<(String, String)>);

/// The lines a code renderer drew for `earlier`'s code, standing in for
/// `code`'s. A block that grew keeps the colors of the lines it had that
/// are complete and shows the rest in `plain` until the renderer answers.
/// A renderer that drew as many lines as the code had not is shown as it
/// drew.
fn code_lines(
    code: &str,
    earlier: &CodeQuestion,
    drawn: &[String],
    plain: Style,
) -> Vec<StyledLine> {
    let parse = |line: &String| yapi_tui::ansi::parse_line(line).0;
    let had = earlier.code.split('\n').count();
    if earlier.code == code || drawn.len() != had {
        return drawn.iter().map(parse).collect();
    }
    // The last line it had may have grown.
    let complete = code.as_bytes().get(earlier.code.len()) == Some(&b'\n');
    let kept = if complete { had } else { had - 1 };
    drawn[..kept]
        .iter()
        .map(parse)
        .chain(
            code.split('\n')
                .skip(kept)
                .map(|line| Line::from(Span::styled(line.to_owned(), plain))),
        )
        .collect()
}

impl super::App {
    /// The Markdown theme the item with `answers` renders with: its code
    /// goes to the extensions' code renderers.
    pub(super) fn item_markdown(&self, answers: &ItemAnswers) -> MarkdownTheme {
        let code = answers.code.clone();
        let plain = self.markdown.code_block;
        MarkdownTheme {
            highlight_code: Some(Arc::new(
                move |text: &str, lang: Option<&str>, width: usize| {
                    let question = CodeQuestion {
                        code: text.to_owned(),
                        lang: lang.map(str::to_owned),
                        width,
                    };
                    code.get(&question, |earlier, drawn| {
                        code_lines(text, earlier, drawn, plain)
                    })
                },
            )),
            ..self.markdown.clone()
        }
    }

    /// Asks the extensions what the items rendered since the last call
    /// left unanswered.
    pub(super) fn ask_drawers(&mut self) {
        for answers in std::mem::take(&mut self.asking) {
            let renderers = self.drawers.code.clone();
            self.ask(&answers.code, code_answers, move |question| {
                let renderers = renderers.clone();
                Box::pin(async move {
                    let request = json!({
                        "code": question.code,
                        "lang": question.lang,
                        "width": question.width,
                    });
                    // The first renderer in load order that draws wins.
                    let mut errors = Vec::new();
                    for renderer in renderers {
                        match renderer.render_code(&request).await {
                            Ok(Some(lines)) => return (Some(lines), errors),
                            Ok(None) => {}
                            Err(error) => errors.push((renderer.source().path, error)),
                        }
                    }
                    (None, errors)
                })
            });
        }
    }

    /// Asks `deferred`'s waiting questions through `answer`, one task each.
    fn ask<Q: Question, A: Clone + Send + 'static>(
        &self,
        deferred: &Deferred<Q, A>,
        kind: Kind<Q, A>,
        answer: impl Fn(Q) -> BoxFuture<'static, Answered<A>>,
    ) {
        for (slot, question) in deferred.questions() {
            let asked = answer(question.clone());
            let (deferred, tx, epoch) = (deferred.clone(), self.tx.clone(), self.epoch);
            tokio::spawn(async move {
                let answered = asked.await;
                let then = move |app: &mut super::App| {
                    app.answered(&deferred, kind, slot, question, answered);
                };
                let _ = tx.send(super::Event::Then(epoch, Box::new(then)));
            });
        }
    }

    /// Takes an answer: redraws the item when it draws differently, and
    /// asks the newer question its slot may have.
    fn answered<Q: Question, A: Clone + Send + 'static>(
        &mut self,
        deferred: &Deferred<Q, A>,
        kind: Kind<Q, A>,
        slot: usize,
        question: Q,
        (answer, errors): Answered<A>,
    ) {
        for (path, error) in errors {
            // Once per extension, as what it fails to draw shows without it.
            if self.drawer_errors.insert(path.clone()) {
                self.extension_error(format!("Extension \"{path}\" error: {error}"), None);
            }
        }
        // Answers forgotten since they were asked are dropped.
        let Some(index) = self
            .answers
            .iter()
            .position(|answers| kind(answers).same(deferred))
        else {
            return;
        };
        if deferred.answer(slot, question, answer) {
            self.touch(index);
        }
        self.asking.push(self.answers[index].clone());
    }

    /// Forgets every answer, as when the theme, the extensions or an
    /// extension runtime change what they would draw.
    pub(super) fn forget_answers(&mut self) {
        for answers in &mut self.answers {
            *answers = ItemAnswers::default();
        }
        self.asking.clear();
        self.invalidate_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Question for String {
        fn continues(&self, earlier: &Self) -> bool {
            self.starts_with(earlier.as_str())
        }
    }

    type Answers = Deferred<String, &'static str>;

    /// A render that reads `questions` in order.
    fn render(answers: &Answers, questions: &[&str]) -> (Vec<Option<&'static str>>, bool) {
        answers.begin();
        let read = questions
            .iter()
            .map(|question| answers.get(&(*question).to_owned(), |_, answer| *answer))
            .collect();
        (read, answers.end())
    }

    #[test]
    fn asks_each_slot_once_and_keeps_its_answer() {
        let answers = Answers::default();
        assert_eq!(render(&answers, &["a", "b"]), (vec![None, None], true));
        assert_eq!(
            answers.questions(),
            [(0, "a".to_owned()), (1, "b".to_owned())]
        );
        // Asked questions are not asked again.
        assert_eq!(render(&answers, &["a", "b"]), (vec![None, None], false));
        assert!(answers.questions().is_empty());
        assert!(answers.answer(0, "a".into(), Some("A")));
        assert!(!answers.answer(1, "b".into(), None), "nothing to draw");
        assert_eq!(
            render(&answers, &["a", "b"]),
            (vec![Some("A"), None], false)
        );
        // A slot the render no longer reads goes.
        assert_eq!(render(&answers, &["a"]), (vec![Some("A")], false));
        assert!(!answers.answer(1, "b".into(), Some("B")));
    }

    #[test]
    fn a_growing_question_keeps_the_previous_answer_and_asks_only_the_newest() {
        let answers = Answers::default();
        render(&answers, &["fn"]);
        answers.questions();
        // Questions that come while one is out wait, and only the newest is
        // asked once it returns.
        render(&answers, &["fn m"]);
        render(&answers, &["fn ma"]);
        assert!(answers.questions().is_empty());
        assert!(answers.answer(0, "fn".into(), Some("FN")));
        assert_eq!(render(&answers, &["fn ma"]), (vec![Some("FN")], true));
        assert_eq!(answers.questions(), [(0, "fn ma".to_owned())]);
        // A question that does not continue the answered one draws plain.
        assert_eq!(render(&answers, &["let"]), (vec![None], false));
        // An answer to a question the slot moved on from still stands in.
        assert!(answers.answer(0, "fn ma".into(), Some("FN MA")));
        assert_eq!(render(&answers, &["fn main"]).0, [Some("FN MA")]);
    }
}
