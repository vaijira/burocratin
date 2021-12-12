use crate::{
    data::FinancialInformation,
    reports::{aeat_720::Aeat720Report, aforix_d6::create_d6_form},
};

use anyhow::{bail, Result};
use js_sys::{Array, Uint8Array};
use wasm_bindgen::JsValue;
use web_sys::{Blob, BlobPropertyBag, Url};

pub fn generate_d6(info: &FinancialInformation, old_path: &str) -> Result<String> {
    let result: String;
    match create_d6_form(info) {
        Ok(d6_form) => {
            let mut blob_properties = BlobPropertyBag::new();
            blob_properties.type_("application/octet-stream");
            let d6_array = Array::new_with_length(1);
            d6_array.set(0, JsValue::from(Uint8Array::from(&d6_form[..])));

            let blob = Blob::new_with_u8_array_sequence_and_options(
                &JsValue::from(d6_array),
                &blob_properties,
            );
            match blob {
                Ok(blob_data) => {
                    if !old_path.is_empty() {
                        if let Err(err) = Url::revoke_object_url(old_path) {
                            log::error!("Error deleting old D6 form: {:?}", err);
                            bail!("Error deleting old D6 form");
                        }
                    }
                    result = Url::create_object_url_with_blob(&blob_data).unwrap();
                }
                Err(err) => {
                    log::error!("Unable to generate d6 form: {:?}", err);
                    bail!("Unable to generate D6 blob");
                }
            }
        }
        Err(err) => {
            log::error!("Unable to generate D6: {}", err);
            bail!("Unable to generate D6");
        }
    }

    Ok(result)
}

pub fn generate_720(info: &FinancialInformation, old_path: &str) -> Result<String> {
    let result;
    let aeat720report = match Aeat720Report::new(info) {
        Ok(report) => report,
        Err(err) => {
            log::error!("Unable to generate Aeat720 report: {}", err);
            bail!("unable to create AEAT 720 report");
        }
    };
    match aeat720report.generate() {
        Ok(aeat720_form) => {
            let mut blob_properties = BlobPropertyBag::new();
            blob_properties.type_("application/octet-stream");
            let aeat720_array = Array::new_with_length(1);
            aeat720_array.set(0, JsValue::from(Uint8Array::from(&aeat720_form[..])));

            let blob = Blob::new_with_u8_array_sequence_and_options(
                &JsValue::from(aeat720_array),
                &blob_properties,
            );
            match blob {
                Ok(blob_data) => {
                    if !old_path.is_empty() {
                        if let Err(err) = Url::revoke_object_url(old_path) {
                            log::error!("Error deleting old aeat 720 form: {:?}", err);
                            bail!("Error deleting old AEAT 720 form");
                        }
                    }
                    result = Url::create_object_url_with_blob(&blob_data).unwrap();
                }
                Err(err) => {
                    log::error!("Unable to generate aeat 720 form: {:?}", err);
                    bail!("Unable to generate AEAT 720 form blob");
                }
            }
        }
        Err(err) => {
            log::error!("Unable to generate Aeat 720 report: {}", err);
            bail!("Unable to generate AEAT 720 from data")
        }
    }

    Ok(result)
}
