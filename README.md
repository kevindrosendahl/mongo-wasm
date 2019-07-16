# mongo-wasm

Tools for creating `$wasm` aggregation pipeline stages.

## Installing

To use, add the following to your `Cargo.toml`:

```toml
[dependencies]
mongo-wasm = { git = "https://github.com/kevindrosendahl/mongo-wasm", branch = "master" }
```

## Usage

```rust
// First, require the prelude:
use mongo_wasm::prelude::*;

// Then, implement your pipeline stage.
// For example, here's a pipeline stage equivalent to `{ $project: { foo: -1 } }`:
struct ProjectPipelineStage {
    iter: Box<dyn Iterator<Item = Document>>,
}

impl PipelineStage for ProjectPipelineStage {
    fn new(document_source: Box<dyn DocumentSource>) -> Self {
        ProjectPipelineStage {
            iter: document_source.iter(),
        }
    }

    fn next(&mut self) -> Option<Document> {
        self.iter.next().map(|mut doc| {
            doc.remove("foo");
            doc
        })
    }
}

// Finally, register your pipeline stage:
mongo_pipeline_stage!(ProjectPipelineStage)
```

That's it! To build your module run:

```bash
$ cargo build --target wasm32-unknown-unknown --release
```

You can also use the `mock_mongo_pipeline_stage!` macro to test your pipeline stage. For example, testing our `$project` stage from above:

```rust
#[test]
fn test_single_doc_pipeline() {
    let mut doc = Document::new();
    doc.insert("hello", "world");
    doc.insert("foo", "bar");
    let mut pipeline_stage = mock_mongo_pipeline_stage!(ProjectPipelineStage, doc);

    let result = pipeline_stage.next();
    assert!(result.is_some());

    let mut expected = Document::new();
    expected.insert("hello", "world");
    assert_eq!(expected, result.unwrap());

    let result = pipeline_stage.next();
    assert!(result.is_none());
}
```
