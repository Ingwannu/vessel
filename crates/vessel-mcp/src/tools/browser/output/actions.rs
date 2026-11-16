//! Foreign result projection consumes completed facts, never input arguments.
use serde_json::{Map, Value, json};
use vessel_proto::browser::{ActionOutcome, BrowserCommand, BrowserId, CssPoint, ElementRef};

pub(super) fn selectors_match(command: &BrowserCommand, outcome: &ActionOutcome) -> bool {
    use ActionOutcome as A;
    use BrowserCommand as C;
    match outcome {
        A::Click { element, .. } => {
            matches!(command, C::Click(args) if &args.target.element == element)
        }
        A::Fill { element } => matches!(command, C::Fill(args) if &args.target.element == element),
        A::Type { element, .. } => matches!(command, C::Type(args) if &args.element == element),
        A::Keypress { key, element, .. } => {
            matches!(command, C::Keypress(args) if &args.key == key && &args.element == element)
        }
        A::Select { element, value } => {
            matches!(command, C::Select(args) if &args.target.element == element && &args.value == value)
        }
        A::Hover { element, .. } => matches!(command, C::Hover(args) if &args.element == element),
        A::Drag { source, target, .. } => {
            matches!(command, C::Drag(args) if &args.source == source && &args.destination == target)
        }
        A::Scroll {
            element,
            delta_x,
            delta_y,
            ..
        } => {
            matches!(command, C::Scroll(args) if &args.element == element && &args.delta_x == delta_x && &args.delta_y == delta_y)
        }
    }
}

pub(super) fn metadata(browser: &BrowserId, outcome: ActionOutcome) -> Value {
    use ActionOutcome as A;
    let mut fields = Map::new();
    put(&mut fields, "browserId", json!(browser));
    match outcome {
        A::Click { element, position } | A::Hover { element, position } => {
            reference(&mut fields, Some(element));
            point(&mut fields, Some(position), Coordinates::Normal);
        }
        A::Fill { element } => reference(&mut fields, Some(element)),
        A::Type { element, position } => {
            reference(&mut fields, element);
            point(&mut fields, position, Coordinates::Normal);
        }
        A::Keypress {
            key,
            element,
            position,
        } => {
            put(&mut fields, "key", json!(key));
            reference(&mut fields, element);
            point(&mut fields, position, Coordinates::Normal);
        }
        A::Select { element, value } => {
            reference(&mut fields, Some(element));
            put(&mut fields, "value", json!(value));
        }
        A::Drag {
            source,
            target,
            source_position,
            target_position,
        } => {
            put(&mut fields, "sourceRef", json!(source));
            put(&mut fields, "targetRef", json!(target));
            point(&mut fields, Some(source_position), Coordinates::Source);
            point(&mut fields, Some(target_position), Coordinates::Target);
        }
        A::Scroll {
            element,
            delta_x,
            delta_y,
            position,
        } => {
            reference(&mut fields, element);
            put(&mut fields, "deltaX", json!(delta_x));
            put(&mut fields, "deltaY", json!(delta_y));
            point(&mut fields, Some(position), Coordinates::Normal);
        }
    }
    Value::Object(fields)
}

#[derive(Clone, Copy)]
enum Coordinates {
    Normal,
    Source,
    Target,
}
fn point(fields: &mut Map<String, Value>, position: Option<CssPoint>, coordinates: Coordinates) {
    if let Some(position) = position {
        let (x, y) = match coordinates {
            Coordinates::Normal => ("x", "y"),
            Coordinates::Source => ("sourceX", "sourceY"),
            Coordinates::Target => ("targetX", "targetY"),
        };
        put(fields, x, json!(position.x));
        put(fields, y, json!(position.y));
    }
}
fn reference(fields: &mut Map<String, Value>, element: Option<ElementRef>) {
    if let Some(element) = element {
        put(fields, "ref", json!(element));
    }
}
fn put(fields: &mut Map<String, Value>, name: &'static str, value: Value) {
    let _previous = fields.insert(name.to_owned(), value);
}
