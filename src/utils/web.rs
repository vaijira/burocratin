use crate::{data::Aeat720Information, reports::aeat_720::Aeat720Report};

use anyhow::{Result, bail};
use js_sys::{Array, Uint8Array};
use wasm_bindgen::JsValue;
use web_sys::{Blob, BlobPropertyBag, Url};

pub fn delete_path(path: String) -> Result<()> {
    if let Err(err) = Url::revoke_object_url(&path) {
        log::error!("Error deleting old aeat 720 form: {err:?}");
        bail!("Error deleting old AEAT 720 form");
    }

    Ok(())
}

pub fn generate_720(info: &Aeat720Information) -> Result<String> {
    let result;
    let aeat720report = match Aeat720Report::new(info) {
        Ok(report) => report,
        Err(err) => {
            log::error!("Unable to generate Aeat720 report: {err}");
            bail!("unable to create AEAT 720 report");
        }
    };
    match aeat720report.generate() {
        Ok(aeat720_form) => {
            let blob_properties = BlobPropertyBag::new();
            blob_properties.set_type("application/octet-stream");
            let aeat720_array = Array::new_with_length(1);
            aeat720_array.set(0, JsValue::from(Uint8Array::from(&aeat720_form[..])));

            let blob = Blob::new_with_u8_array_sequence_and_options(
                &JsValue::from(aeat720_array),
                &blob_properties,
            );
            match blob {
                Ok(blob_data) => {
                    result = Url::create_object_url_with_blob(&blob_data).unwrap();
                }
                Err(err) => {
                    log::error!("Unable to generate aeat 720 form: {err:?}");
                    bail!("Unable to generate AEAT 720 form blob");
                }
            }
        }
        Err(err) => {
            log::error!("Unable to generate Aeat 720 report: {err}");
            bail!("Unable to generate AEAT 720 from data")
        }
    }

    Ok(result)
}
