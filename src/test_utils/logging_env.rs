use tracing_subscriber::prelude::*;

// A helper that allows to skip spans without events.
struct FilteredProcessor<P>(P);

impl<P: tracing_forest::Processor> tracing_forest::Processor for FilteredProcessor<P> {
    fn process(&self, tree: tracing_forest::tree::Tree) -> tracing_forest::processor::Result {
        fn is_used(tree: &tracing_forest::tree::Tree) -> bool {
            match tree {
                tracing_forest::tree::Tree::Span(span) => span.nodes().iter().any(is_used),
                tracing_forest::tree::Tree::Event(_) => true,
            }
        }

        if is_used(&tree) {
            self.0.process(tree)
        } else {
            Ok(())
        }
    }
}

fn target_icon(target: &str) -> Option<char> {
    let target_parts = target.split("::").collect::<Vec<_>>();

    Some(match target_parts.as_slice() {
        ["discovery", ..] | [_, "discovery", ..] => '🔍',
        ["client", ..] => '📡',
        ["server", ..] => '🏭',
        ["conformu" | "test_utils", ..] => '🧪',
        _ => return None,
    })
}

fn target_tag(event: &tracing::Event<'_>) -> Option<tracing_forest::Tag> {
    let target = event.metadata().target().strip_prefix("ascom_alpaca::")?;

    let mut builder = tracing_forest::Tag::builder()
        .prefix(target)
        .level(*event.metadata().level());

    if let Some(icon) = target_icon(target) {
        builder = builder.icon(icon);
    }

    Some(builder.build())
}

#[ctor::ctor]
fn prepare_test_env() {
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "full");
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_target("ascom_alpaca", tracing::Level::INFO)
                .with_target("ascom_alpaca::conformu", tracing::Level::TRACE),
        )
        .with(tracing_forest::ForestLayer::new(
            FilteredProcessor(tracing_forest::printer::TestCapturePrinter::new()),
            target_tag,
        ))
        .with(tracing_error::ErrorLayer::default())
        .init();

    color_eyre::config::HookBuilder::default()
        .add_frame_filter(Box::new(|frames| {
            frames.retain(|frame| {
                frame.filename.as_ref().map_or(false, |filename| {
                    // Only keep our own files in the backtrace to reduce noise.
                    filename.starts_with(env!("CARGO_MANIFEST_DIR"))
                })
            });
        }))
        .install()
        .expect("Failed to install color_eyre");
}
