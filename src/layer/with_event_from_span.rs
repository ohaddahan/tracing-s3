/// Macro for creating synthetic events from span data.
/// 
/// This macro creates a new tracing event that appears to come from a specific span,
/// allowing for rich span lifecycle logging with custom fields.
/// 
/// # Arguments
/// * `$id` - The span ID
/// * `$span` - The span reference
/// * `$field:literal = $value:expr` - Field-value pairs to include in the event
/// * `|$event| $code:block` - Closure that receives the created event
/// 
/// # Example
/// ```rust
/// with_event_from_span!(
///     span_id, 
///     span_ref, 
///     "message" = "span_entered", 
///     "duration" = elapsed_time,
///     |event| {
///         // Handle the synthetic event
///         self.on_event(&event, ctx);
///     }
/// );
/// ```
#[macro_export]
macro_rules! with_event_from_span {
    ($id:ident, $span:ident, $($field:literal = $value:expr),*, |$event:ident| $code:block) => {
        let meta = $span.metadata();
        let cs = meta.callsite();
        let fs = field::FieldSet::new(&[$($field),*], cs);
        #[allow(unused)]
        let mut iter = fs.iter();
        let v = [$(
            (&iter.next().unwrap(), ::core::option::Option::Some(&$value as &dyn field::Value)),
        )*];
        let vs = fs.value_set(&v);
        let $event = Event::new_child_of($id, meta, &vs);
        $code
    };
}
