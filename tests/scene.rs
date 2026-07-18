use olydraw::scene::{ElementKind, Point, Scene, ShapeKind, Style};

fn style() -> Style {
    Style {
        color: [255, 0, 0, 255],
        stroke_width: 2.0,
    }
}

fn rect(x1: f32, y1: f32, x2: f32, y2: f32) -> ElementKind {
    ElementKind::Shape {
        kind: ShapeKind::Rect,
        start: Point::new(x1, y1),
        end: Point::new(x2, y2),
    }
}

fn ids(scene: &Scene) -> Vec<u64> {
    scene.elements().iter().map(|e| e.id).collect()
}

#[test]
fn added_elements_are_visible_in_order() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    let b = scene.add(rect(20.0, 20.0, 30.0, 30.0), style());
    assert_eq!(ids(&scene), vec![a, b]);
}

#[test]
fn remove_deletes_whole_elements_and_ignores_unknown_ids() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    let b = scene.add(rect(20.0, 20.0, 30.0, 30.0), style());
    scene.remove(&[a, 9999]);
    assert_eq!(ids(&scene), vec![b]);
}

#[test]
fn translate_moves_element_geometry() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    scene.translate(&[a], 5.0, 7.0);
    match &scene.elements()[0].kind {
        ElementKind::Shape { start, end, .. } => {
            assert_eq!((start.x, start.y), (5.0, 7.0));
            assert_eq!((end.x, end.y), (15.0, 17.0));
        }
        other => panic!("unexpected kind {other:?}"),
    }
}

#[test]
fn undo_and_redo_round_trip_add() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    scene.undo();
    assert!(scene.elements().is_empty());
    scene.redo();
    assert_eq!(ids(&scene), vec![a]);
}

#[test]
fn undo_and_redo_round_trip_remove() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    let b = scene.add(rect(20.0, 20.0, 30.0, 30.0), style());
    scene.remove(&[a]);
    assert_eq!(ids(&scene), vec![b]);
    scene.undo();
    assert_eq!(ids(&scene), vec![a, b]);
    scene.redo();
    assert_eq!(ids(&scene), vec![b]);
}

#[test]
fn undo_and_redo_round_trip_translate() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    scene.translate(&[a], 5.0, 5.0);
    scene.undo();
    match &scene.elements()[0].kind {
        ElementKind::Shape { start, .. } => assert_eq!((start.x, start.y), (0.0, 0.0)),
        other => panic!("unexpected kind {other:?}"),
    }
    scene.redo();
    match &scene.elements()[0].kind {
        ElementKind::Shape { start, .. } => assert_eq!((start.x, start.y), (5.0, 5.0)),
        other => panic!("unexpected kind {other:?}"),
    }
}

#[test]
fn clear_removes_everything_and_is_a_single_undo_step() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    let b = scene.add(rect(20.0, 20.0, 30.0, 30.0), style());
    scene.clear();
    assert!(scene.elements().is_empty());
    scene.undo();
    assert_eq!(ids(&scene), vec![a, b]);
    scene.redo();
    assert!(scene.elements().is_empty());
}

#[test]
fn clear_on_empty_scene_pushes_no_undo_entry() {
    let mut scene = Scene::new();
    scene.clear();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    scene.undo();
    assert!(
        scene.elements().is_empty(),
        "undo should revert the add, not a phantom clear"
    );
    scene.redo();
    assert_eq!(ids(&scene), vec![a]);
}

#[test]
fn undo_past_empty_history_is_a_noop() {
    let mut scene = Scene::new();
    scene.undo();
    scene.undo();
    assert!(scene.elements().is_empty());
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    assert_eq!(ids(&scene), vec![a]);
}

#[test]
fn redo_with_no_undone_commands_is_a_noop() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    scene.redo();
    assert_eq!(ids(&scene), vec![a]);
}

#[test]
fn new_command_truncates_redo_stack() {
    let mut scene = Scene::new();
    let a = scene.add(rect(0.0, 0.0, 10.0, 10.0), style());
    let _b = scene.add(rect(20.0, 20.0, 30.0, 30.0), style());
    scene.undo();
    let c = scene.add(rect(40.0, 40.0, 50.0, 50.0), style());
    scene.redo();
    assert_eq!(
        ids(&scene),
        vec![a, c],
        "redo after new command must do nothing"
    );
}

#[test]
fn history_is_capped_and_drops_oldest_entries() {
    let mut scene = Scene::new();
    for i in 0..105 {
        let o = i as f32;
        scene.add(rect(o, o, o + 1.0, o + 1.0), style());
    }
    for _ in 0..200 {
        scene.undo();
    }
    assert_eq!(
        scene.elements().len(),
        5,
        "only the newest 100 adds should be undoable"
    );
}
