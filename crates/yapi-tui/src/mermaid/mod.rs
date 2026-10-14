//! Port of `src/index.ts` and `src/types.ts` of grok-mermaid 0.2.3 (Apache-2.0).
//!
//! Renders a Mermaid source block as Unicode box-drawing art. Supported:
//! `graph`/`flowchart` (including `subgraph`), `stateDiagram`, `classDiagram`,
//! `erDiagram` and `sequenceDiagram`.

mod canvas;
mod graph;
mod labels;
mod layout;
mod layout_seq;
mod parse;
mod transform;
mod width;
mod width_data;

pub use transform::transform;

use canvas::Canvas;
use layout::{CanvasResult, layout_class, layout_flowchart, layout_grouped};
use layout_seq::layout_sequence;
use parse::{
    DiagramKind, diagram_kind, parse_class, parse_er, parse_graph, parse_sequence, parse_state,
};

/// Semantic class of a run of cells. The renderer never knows about colour;
/// consumers map these to their own theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cls {
    /// Box outlines, subgraph frames, compartment rules.
    Border,
    /// Node, participant and compartment labels.
    Text,
    /// Connector lines and arrowheads.
    Edge,
    /// Text sitting on an edge.
    EdgeLabel,
    /// The header of a source box. Never produced by `render`.
    Title,
    /// Blank filler.
    None,
}

/// A run of adjacent cells sharing one semantic class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The cells' text.
    pub text: String,
    /// Their class.
    pub cls: Cls,
}

/// A rendered diagram.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MermaidArt {
    /// The rows, right-trimmed.
    pub plain: Vec<String>,
    /// The same rows as runs of cells, to color them.
    pub styled: Vec<Vec<Span>>,
    /// The display columns of the widest row, to compare with the space
    /// available.
    pub width: usize,
    /// What the flowchart grammar could not read and dropped: the art is real
    /// but incomplete. The other grammars refuse the whole diagram instead.
    /// A diagram being typed warns at nearly every intermediate state.
    pub warnings: Vec<String>,
}

/// Render a Mermaid source block as Unicode box-drawing art.
///
/// The diagram is laid out at whatever size it needs; `width` reports the
/// columns that turned out to be. Deciding what to do when that exceeds the
/// space at hand is the caller's.
///
/// `None` means there is no art to show: blank input, a syntax error, a
/// diagram type this renderer does not draw, or one large enough that laying it
/// out is refused.
///
/// Rendering is best-effort. A flowchart keeps whatever parsed; the stricter
/// grammars additionally get one retry without their final line, which keeps a
/// streaming diagram on screen while its last statement is half-typed.
/// Everything given up on is listed in `warnings`.
pub fn render(src: &str) -> Option<MermaidArt> {
    let src = labels::strip_controls(src);
    if labels::js_trim(&src).is_empty() {
        return None;
    }
    let drawn = attempt(&src)?;
    let lines = drawn.canvas.to_lines();
    Some(MermaidArt {
        plain: lines.plain,
        styled: lines.styled,
        width: lines.width,
        warnings: drawn.warnings,
    })
}

struct Drawn {
    canvas: Canvas,
    warnings: Vec<String>,
}

/// Draw `src`, retrying once without its last line if the grammar rejects it.
///
/// State, class, ER and sequence fail a whole diagram on one unreadable
/// statement, and while a source is streaming its last line is usually still
/// being typed. Without this a diagram would alternate with the source all the
/// way in. Only the final line is dropped, and doing so is always reported, so
/// a finished document with a bad last line still says what it lost rather than
/// quietly rendering short.
fn attempt(src: &str) -> Option<Drawn> {
    if let Some(drawn) = draw(src) {
        return Some(drawn);
    }

    let body = labels::js_trim_end(src);
    let cut = body.rfind('\n')?;
    let mut salvaged = draw(&body[..cut])?;

    let dropped = labels::js_trim(&body[cut + 1..]);
    salvaged
        .warnings
        .push(format!("dropped, unreadable final line: \"{dropped}\""));
    Some(salvaged)
}

/// Dispatch on the declared diagram type; `None` means nothing was drawn.
fn draw(src: &str) -> Option<Drawn> {
    let plain = |canvas: CanvasResult| {
        canvas.map(|canvas| Drawn {
            canvas,
            warnings: Vec::new(),
        })
    };

    match diagram_kind(src)? {
        DiagramKind::Flowchart => {
            let graph = parse_graph(src)?;
            let canvas = if graph.groups.is_empty() {
                layout_flowchart(&graph)
            } else {
                layout_grouped(&graph)
            }?;
            Some(Drawn {
                canvas,
                warnings: graph.warnings,
            })
        }
        DiagramKind::State => plain(layout_flowchart(&parse_state(src)?)),
        DiagramKind::Class => {
            let (graph, infos) = parse_class(src)?;
            plain(layout_class(&graph, &infos))
        }
        DiagramKind::Er => {
            let (graph, infos) = parse_er(src)?;
            plain(layout_class(&graph, &infos))
        }
        DiagramKind::Sequence => plain(layout_sequence(&parse_sequence(src)?)),
    }
}
