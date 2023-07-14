// Copyright 2023 WHERE TRUE Technologies.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    ffi::{c_char, CStr, CString},
    sync::Arc,
};

use arrow::ffi_stream::FFI_ArrowArrayStream as ArrowArrayStream;
use datafusion::prelude::{SessionConfig, SessionContext};
use exon::{context::ExonSessionExt, ffi::create_dataset_stream_from_table_provider};
use tokio::runtime::Runtime;

#[repr(C)]
pub struct VCFReaderResult {
    error: *const c_char,
}

#[no_mangle]
pub unsafe extern "C" fn vcf_query_reader(
    stream_ptr: *mut ArrowArrayStream,
    uri: *const c_char,
    query: *const c_char,
    batch_size: usize,
) -> VCFReaderResult {
    let uri = match CStr::from_ptr(uri).to_str() {
        Ok(uri) => uri,
        Err(e) => {
            let error = CString::new(format!("could not parse uri: {}", e)).unwrap();
            return VCFReaderResult {
                error: error.into_raw(),
            };
        }
    };

    let rt = Arc::new(Runtime::new().unwrap());

    let config = SessionConfig::new().with_batch_size(batch_size);
    let ctx = SessionContext::with_config_exon(config);

    let query = match CStr::from_ptr(query).to_str() {
        Ok(query) => query,
        Err(e) => {
            let error = CString::new(format!("could not parse query: {}", e)).unwrap();
            return VCFReaderResult {
                error: error.into_raw(),
            };
        }
    };

    rt.block_on(async {
        let df = match ctx.query_vcf_file(uri, query).await {
            Ok(df) => df,
            Err(e) => {
                let error = CString::new(format!("could not read VCF file: {}", e)).unwrap();
                return VCFReaderResult {
                    error: error.into_raw(),
                };
            }
        };

        match create_dataset_stream_from_table_provider(df, rt.clone(), stream_ptr).await {
            Ok(_) => VCFReaderResult {
                error: std::ptr::null(),
            },
            Err(e) => {
                let error =
                    CString::new(format!("could not create dataset stream: {}", e)).unwrap();
                return VCFReaderResult {
                    error: error.into_raw(),
                };
            }
        }
    })
}