pub mod prelude {
    pub use super::{
        mock::MockDocumentSource, mock_mongo_pipeline_stage, mongo_pipeline_stage,
        wasm::WasmDocumentSource, DocumentSource, PipelineStage,
    };

    pub use bson;
    pub use bson::Document;
}

/// Represents a aggregation pipeline stage, which can use its DocumentSource
/// to retrieve documents from the prior stage, and returns documents via next().
/// Must be implemented by the user.
pub trait PipelineStage {
    fn new(document_source: Box<dyn DocumentSource>) -> Self;

    fn next(&mut self) -> Option<bson::Document>;
}

/// Represents the prior stage in the aggregation pipeline. The iterator can be
/// used to retrieve the next document from the prior stage. Not implemented
/// by the user.
pub trait DocumentSource {
    fn iter(&self) -> Box<dyn Iterator<Item = bson::Document>>;
}

/// Utilities for creating WASM target pipeline stages.
#[doc(hidden)]
pub mod wasm {
    use bson::Document;

    #[no_mangle]
    extern "C" {
        fn __mongo_pipeline_document_source_next() -> i32;
    }

    pub struct WasmDocumentSource;

    impl super::DocumentSource for WasmDocumentSource {
        fn iter(&self) -> Box<dyn Iterator<Item = Document>> {
            Box::new(WasmDocumentSourceIterator {})
        }
    }

    pub struct WasmDocumentSourceIterator;

    impl Iterator for WasmDocumentSourceIterator {
        type Item = bson::Document;

        fn next(&mut self) -> Option<Self::Item> {
            let base_ptr = unsafe { __mongo_pipeline_document_source_next() } as *const i32;
            if base_ptr.is_null() {
                return None;
            }

            let len = unsafe { *base_ptr };
            assert!(len >= 0);

            let buf_ptr = unsafe { base_ptr.offset(1) } as *const u8;
            let mut buf = unsafe { std::slice::from_raw_parts(buf_ptr, len as usize) };
            Some(bson::decode_document(&mut buf).unwrap())
        }
    }

    #[macro_export]
    macro_rules! mongo_pipeline_stage {
        ( $ty:ty ) => {
            fn __assert_valid_pipeline()
            where
                $ty: PipelineStage,
            {
                // This error means that your supplied type does not implement mongodb_wasm::PipelineStage.
            }

		    thread_local!(static PIPELINE_STAGE: std::cell::RefCell<$ty> = std::cell::RefCell::new(<$ty>::new(Box::new(WasmDocumentSource {}))));

            #[no_mangle]
            unsafe extern "C" fn __mongo_pipeline_stage_next() -> i32 {
                PIPELINE_STAGE.with(|pipeline_stage| {
                    match pipeline_stage.borrow_mut().next() {
                        Some(doc) => {
                            let mut buf = Vec::new();
                            bson::encode_document(&mut buf, &doc).unwrap();

                            let ptr = buf.as_ptr();
                            std::mem::forget(buf);
                            ptr as i32
                        }
                        None => 0,
                    }
		        })
            }
        };
    }
}

/// Utilities for testing PipelineStages.
#[doc(hidden)]
pub mod mock {
    use bson::Document;

    pub struct MockDocumentSource {
        docs: Vec<Document>,
    }

    impl MockDocumentSource {
        pub fn new(docs: Vec<Document>) -> MockDocumentSource {
            MockDocumentSource { docs }
        }
    }

    impl super::DocumentSource for MockDocumentSource {
        fn iter(&self) -> Box<dyn Iterator<Item = Document>> {
            Box::new(self.docs.clone().into_iter())
        }
    }

    #[macro_export]
    macro_rules! mock_mongo_pipeline_stage {
         ( $ty:ty ) => {
            <$ty>::new(Box::new(MockDocumentSource::new(Vec::new())))
        };
        ( $ty:ty, $( $doc:expr ),* ) => {
            {
                let mut docs = Vec::new();
                $(
                    docs.push($doc);
                )*

                <$ty>::new(Box::new(MockDocumentSource::new(docs)))
            }
        };
    }
}
