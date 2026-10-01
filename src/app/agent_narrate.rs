//! LCV-144 — how the agent narrates the live document (ADR 0007 §D5, ADR 0010 §9).
//!
//! Split out of `agent_apply.rs` unchanged, so the apply arms stay under the
//! 300-LOC cap. Every string here reads the document; none mutates it.

use crate::document::{Document, Entity};

/// A point in the one format every agent-facing string uses: mm, 3 decimals.
pub(super) fn pt(x: f64, y: f64) -> String {
    format!("({x:.3}, {y:.3})")
}

/// An arc's sweep, converted back to the degrees the operator dictated.
/// Radians are the kernel's unit; degrees are the presentation unit.
pub(super) fn sweep(start: f64, end: f64, ccw: bool) -> String {
    let dir = if ccw { "ccw" } else { "cw" };
    format!("{:.1}°→{:.1}° {dir}", start.to_degrees(), end.to_degrees())
}

/// The one word that says what an entity is.
pub(super) fn kind(entity: &Entity) -> &'static str {
    match entity {
        Entity::Line(_) => "line",
        Entity::Circle(_) => "circle",
        Entity::Arc(_) => "arc",
        Entity::Ellipse(_) => "ellipse",
        Entity::Bezier(_) => entity.kind_name(),
    }
}

/// An entity's mm geometry, without its kind. Split from [`describe`] because
/// the two readers want different punctuation: a listing row reads
/// `0: line (0, 0) → …`, while a delete reads `Deleted entity 0 (line, …)`.
pub(super) fn geometry(entity: &Entity) -> String {
    match entity {
        Entity::Line(l) => format!("{} → {} mm", pt(l.p1.x, l.p1.y), pt(l.p2.x, l.p2.y)),
        Entity::Circle(c) => format!(
            "center {} mm, r = {:.3} mm",
            pt(c.center.x, c.center.y),
            c.r
        ),
        Entity::Arc(a) => format!(
            "center {} mm, r = {:.3} mm, {}",
            pt(a.center.x, a.center.y),
            a.r,
            sweep(a.start_angle, a.end_angle, a.ccw)
        ),
        Entity::Ellipse(e) => {
            let span = e.span.map_or(String::new(), |s| {
                format!(", {}", sweep(s.start, s.end, s.ccw))
            });
            format!(
                "center {} mm, rx = {:.3}, ry = {:.3} mm, rotation_deg = {:.3}{span}",
                pt(e.center.x, e.center.y),
                e.rx,
                e.ry,
                e.rotation.to_degrees()
            )
        }
        // LCV-177 T24: the points arrive with the narration arm.
        Entity::Bezier(_) => String::new(),
    }
}

/// One phrase naming an entity's kind and its mm geometry, so a wrong target is
/// legible in the transcript instead of invisible (ADR 0007 §D5).
pub(super) fn describe(entity: &Entity) -> String {
    format!("{}, {}", kind(entity), geometry(entity))
}

/// The bed, in the same mm-to-3-decimals format as everything else.
///
/// Read from the live [`Document::bed_mm`], never from the default constant:
/// an operator who set a 300 × 200 bed and asks the agent to fill it must not
/// be told about a 400 × 400 one (AC 14).
pub(super) fn bed_line(doc: &Document) -> String {
    format!("Bed {:.3} × {:.3} mm.", doc.bed_mm[0], doc.bed_mm[1])
}

/// Every layer in order, the current one marked (LCV-156, ADR 0012 §6).
pub(super) fn layers_line(doc: &Document) -> String {
    let names: Vec<String> = doc
        .layers()
        .iter()
        .map(|l| match l.id == doc.current_layer() {
            true => format!("{} (current)", l.name),
            false => l.name.clone(),
        })
        .collect();
    format!("Layers: {}.", names.join(", "))
}

/// The name of the layer entity `index` sits on.
fn layer_name(doc: &Document, index: usize) -> &str {
    doc.entity_layer(index)
        .and_then(|id| doc.layer(id))
        .map_or("?", |l| l.name.as_str())
}

/// `QueryEntities`: the whole drawing, one entity per line, indices first.
///
/// The header says the count even when it is zero — the model needs to know
/// that it looked and found nothing, which reads differently from a tool that
/// failed to answer.
pub(super) fn list_entities(doc: &Document) -> String {
    if doc.entities.is_empty() {
        let (bed, layers) = (bed_line(doc), layers_line(doc));
        return format!("The drawing is empty (0 entities). {bed}\n{layers}");
    }
    let mut out = format!(
        "The drawing has {} entities. {}\n{}",
        doc.entity_count(),
        bed_line(doc),
        layers_line(doc)
    );
    for (i, entity) in doc.entities.iter().enumerate() {
        let layer = layer_name(doc, i);
        out.push_str(&format!(
            "\n{i}: {} {} layer {layer}",
            kind(entity),
            geometry(entity)
        ));
    }
    out
}

/// `QuerySelection`: which indices are selected, in ascending order.
///
/// `Selection` iterates a `HashSet`, whose order is not guaranteed, so the
/// indices are sorted before they are rendered — otherwise the same selection
/// would narrate differently from run to run.
pub(super) fn list_selection(doc: &Document) -> String {
    let mut indices: Vec<usize> = doc.selection.iter().collect();
    indices.sort_unstable();
    if indices.is_empty() {
        return format!(
            "Nothing is selected (the drawing has {} entities).",
            doc.entity_count()
        );
    }
    let list: Vec<String> = indices.iter().map(|i| i.to_string()).collect();
    format!(
        "{} of {} entities are selected: {}.",
        indices.len(),
        doc.entity_count(),
        list.join(", ")
    )
}

/// The outcome of a `create_drawing` batch of `n` entities appended at `first`,
/// with the post-commit `count` and `revision` (LCV-144 AC 8). Zero-based.
pub(super) fn batch_created(n: usize, first: usize, count: usize, revision: u64) -> String {
    let created = if n == 1 {
        format!("Created 1 entity (index {first}).")
    } else {
        format!(
            "Created {n} entities (indices {first}..={}).",
            first + n.saturating_sub(1)
        )
    };
    format!("{created} The drawing now has {count} entities. Revision {revision}.")
}
