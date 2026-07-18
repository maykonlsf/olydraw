mod element;
mod history;
mod hit;

pub use element::{Element, ElementId, ElementKind, Point, Rect, ShapeKind, Style};

use history::{Command, History};

/// Vector scene: ordered elements (back to front) with undo/redo history.
#[derive(Default)]
pub struct Scene {
    elements: Vec<Element>,
    next_id: ElementId,
    history: History,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an element, returns its id. Pushes one undo entry.
    pub fn add(&mut self, kind: ElementKind, style: Style) -> ElementId {
        self.next_id += 1;
        let element = Element {
            id: self.next_id,
            kind,
            style,
        };
        self.history.push(Command::Add {
            element: element.clone(),
        });
        self.elements.push(element);
        self.next_id
    }

    /// Removes the given elements. Pushes one undo entry. Unknown ids are ignored.
    pub fn remove(&mut self, ids: &[ElementId]) {
        let removed: Vec<(usize, Element)> = self
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| ids.contains(&e.id))
            .map(|(i, e)| (i, e.clone()))
            .collect();
        if removed.is_empty() {
            return;
        }
        self.elements.retain(|e| !ids.contains(&e.id));
        self.history.push(Command::Remove { removed });
    }

    /// Moves the given elements by (dx, dy). Pushes one undo entry.
    pub fn translate(&mut self, ids: &[ElementId], dx: f32, dy: f32) {
        let ids: Vec<ElementId> = self
            .elements
            .iter()
            .filter(|e| ids.contains(&e.id))
            .map(|e| e.id)
            .collect();
        if ids.is_empty() {
            return;
        }
        self.apply_translate(&ids, dx, dy);
        self.history.push(Command::Translate { ids, dx, dy });
    }

    /// Removes all elements. Pushes one undo entry; no-op on empty scene.
    pub fn clear(&mut self) {
        if self.elements.is_empty() {
            return;
        }
        let removed = std::mem::take(&mut self.elements);
        self.history.push(Command::Clear { removed });
    }

    pub fn undo(&mut self) {
        let Some(cmd) = self.history.pop_undo() else {
            return;
        };
        match cmd {
            Command::Add { element } => self.elements.retain(|e| e.id != element.id),
            Command::Remove { removed } => {
                for (index, element) in removed {
                    let index = index.min(self.elements.len());
                    self.elements.insert(index, element);
                }
            }
            Command::Translate { ids, dx, dy } => self.apply_translate(&ids, -dx, -dy),
            Command::Clear { removed } => self.elements = removed,
        }
    }

    pub fn redo(&mut self) {
        let Some(cmd) = self.history.pop_redo() else {
            return;
        };
        match cmd {
            Command::Add { element } => self.elements.push(element),
            Command::Remove { removed } => {
                let ids: Vec<ElementId> = removed.iter().map(|(_, e)| e.id).collect();
                self.elements.retain(|e| !ids.contains(&e.id));
            }
            Command::Translate { ids, dx, dy } => self.apply_translate(&ids, dx, dy),
            Command::Clear { .. } => self.elements.clear(),
        }
    }

    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    /// Topmost element whose stroke is within its hit tolerance of `p`.
    pub fn hit_top(&self, p: Point) -> Option<ElementId> {
        self.elements
            .iter()
            .rev()
            .find(|e| hit::hits_point(e, p))
            .map(|e| e.id)
    }

    /// All elements intersecting the rectangle (marquee selection).
    pub fn hit_rect(&self, rect: Rect) -> Vec<ElementId> {
        self.elements
            .iter()
            .filter(|e| hit::hits_rect(e, rect))
            .map(|e| e.id)
            .collect()
    }

    /// All elements whose stroke intersects segment a-b (eraser drag step).
    pub fn hit_segment(&self, a: Point, b: Point) -> Vec<ElementId> {
        self.elements
            .iter()
            .filter(|e| hit::hits_segment(e, a, b))
            .map(|e| e.id)
            .collect()
    }

    fn apply_translate(&mut self, ids: &[ElementId], dx: f32, dy: f32) {
        for e in self.elements.iter_mut().filter(|e| ids.contains(&e.id)) {
            match &mut e.kind {
                ElementKind::Freehand { points } => {
                    for p in points {
                        p.x += dx;
                        p.y += dy;
                    }
                }
                ElementKind::Shape { start, end, .. } => {
                    start.x += dx;
                    start.y += dy;
                    end.x += dx;
                    end.y += dy;
                }
                ElementKind::Text { pos, .. } => {
                    pos.x += dx;
                    pos.y += dy;
                }
            }
        }
    }
}
