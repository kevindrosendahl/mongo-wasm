pub mod prelude {
    pub use super::{
        mongo_pipeline_stage,
        wasm::{get_input_doc, return_doc, GetNextResultDocument},
        GetNextResult, PipelineStage,
    };

    pub use bson;
    pub use bson::Document;
}

pub enum GetNextResult {
    DocumentReady(bson::Document),
    NeedsNextDocument,
    EOF,
}

/// Represents a aggregation pipeline stage, which can use its DocumentSource
/// to retrieve documents from the prior stage, and returns documents via next().
/// Must be implemented by the user.
pub trait PipelineStage: Default {
    fn get_next(&mut self, doc: Option<bson::Document>) -> GetNextResult;
}

/// Utilities for creating WASM target pipeline stages.
#[doc(hidden)]
pub mod wasm {

    use bson::{decode_document, encode_document, Document};
    use serde::{Deserialize, Serialize};
    pub use std::ffi::c_void;
    pub use std::io::Cursor;

    #[derive(Deserialize, Serialize, Debug)]
    pub struct GetNextResultDocument {
        pub is_eof: bool,
        pub next_doc: Option<bson::Document>,
    }

    #[no_mangle]
    extern "C" {
        fn returnValue(ptr: *mut c_void, len: i32);
        fn getInputSize() -> i32;
        fn writeInputToLocation(ptr: *mut c_void);
    }

    pub fn get_input_doc() -> Option<Document> {
        // Read input into a buffer.
        let input_size = unsafe { getInputSize() } as usize;
        if input_size == 0 {
            return None;
        }

        let buf: Vec<u8> = vec![0; input_size];
        let ptr = buf.as_ptr();
        unsafe { writeInputToLocation(ptr as *mut c_void) };

        // Convert it into a bson::Document.
        Some(decode_document(&mut Cursor::new(&buf[..])).expect("failed to decode input document"))
    }

    pub fn return_doc(doc: Document) {
        let mut buf = Vec::new();
        encode_document(&mut buf, &doc).expect("failed to encode result document");

        let ptr = buf.as_ptr() as *mut c_void;
        unsafe { returnValue(ptr, buf.len() as i32) };
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

		    thread_local!(static PIPELINE_STAGE: std::cell::RefCell<$ty> = std::cell::RefCell::new(<$ty>::default()));

            #[no_mangle]
            unsafe extern "C" fn getNext() {
                PIPELINE_STAGE.with(|pipeline_stage| {
                    let doc = get_input_doc();

                    // Call get_next() and convert it to the proper response to bubble back to mongod.
                    let (is_eof, next_doc) = match pipeline_stage.borrow_mut().get_next(doc) {
                        GetNextResult::DocumentReady(doc) => (false, Some(doc)),
                        GetNextResult::NeedsNextDocument  => (false, None),
                        GetNextResult::EOF => (true, None),
                    };

                    let result = GetNextResultDocument {
                        is_eof, next_doc
                    };

                    let result_doc = bson::to_bson(&result).expect("unable to serialize result");
                    let result_doc = match result_doc {
                        bson::Bson::Document(doc) => doc,
                        _ => panic!("result did not serialize into a document"),
                    };

                    return_doc(result_doc);
		        })
            }
        };
    }
}
