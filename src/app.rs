use std::vec;

use crate::reports::aeat720::Aeat720Report;
use crate::{
    account_notes::AccountNotes, d6_filler::create_d6_form, parsers::degiro_parser::DegiroParser,
    parsers::ib_parser::IBParser,
};
use crate::{account_notes::BalanceNotes, pdf_parser::read_pdf, zip_parser::read_zip_str};

use js_sys::{Array, Uint8Array};
use wasm_bindgen::JsValue;
use web_sys::{Blob, BlobPropertyBag, Url};
use yew::prelude::*;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew_styles::forms::{
    form_file::FormFile,
    form_group::{FormGroup, Orientation},
    form_label::FormLabel,
};
use yew_styles::layouts::{
    container::{Container, Direction, Wrap},
    item::{AlignSelf, Item, ItemLayout},
};

pub struct App {
    tasks: Vec<ReaderTask>,
    degiro_balance_notes: BalanceNotes,
    degiro_account_notes: AccountNotes,
    d6_form_path: String,
    aeat720_form_path: String,
    link: ComponentLink<Self>,
}

pub enum Msg {
    GenerateD6,
    GenerateAeat720,
    UploadDegiroFile(File),
    UploadIBFile(File),
    UploadedDegiroFile(FileData),
    UploadedIBFile(FileData),
    ErrorUploadPdf,
    ErrorUploadZip,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        log::debug!("App created");
        Self {
            tasks: vec![],
            degiro_balance_notes: vec![],
            degiro_account_notes: vec![],
            d6_form_path: "".to_string(),
            aeat720_form_path: "".to_string(),
            link,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        match message {
            Msg::GenerateD6 => {
                match create_d6_form(&self.degiro_balance_notes, "NL") {
                    Ok(d6_form) => {
                        let mut blob_properties = BlobPropertyBag::new();
                        blob_properties.type_("application/octet-stream");
                        let d6_array = Array::new_with_length(1);
                        d6_array.set(0, JsValue::from(Uint8Array::from(&d6_form[..])));
                        //let text = str::from_utf8(&d6_form[..]).unwrap();
                        let blob = Blob::new_with_u8_array_sequence_and_options(
                            &JsValue::from(d6_array),
                            &blob_properties,
                        );
                        match blob {
                            Ok(blob_data) => {
                                if !self.d6_form_path.is_empty() {
                                    if let Err(err) = Url::revoke_object_url(&self.d6_form_path) {
                                        log::error!("Error deleting old D6 form: {:?}", err);
                                    }
                                }
                                self.d6_form_path =
                                    Url::create_object_url_with_blob(&blob_data).unwrap();
                            }
                            Err(err) => log::error!("Unable to generate d6 form: {:?}", err),
                        }
                    }
                    Err(err) => log::error!("Unable to generate D6: {}", err),
                }
                true
            }
            Msg::GenerateAeat720 => {
                let aeat720report = match Aeat720Report::new(
                    &self.degiro_balance_notes,
                    &self.degiro_account_notes,
                    2020,
                    "0",
                    "",
                ) {
                    Ok(report) => report,
                    Err(err) => {
                        log::error!("Unable to generate Aeat720 report: {}", err);
                        return true;
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
                                if !self.aeat720_form_path.is_empty() {
                                    if let Err(err) =
                                        Url::revoke_object_url(&self.aeat720_form_path)
                                    {
                                        log::error!("Error deleting old aeat 720 form: {:?}", err);
                                    }
                                }
                                self.aeat720_form_path =
                                    Url::create_object_url_with_blob(&blob_data).unwrap();
                            }
                            Err(err) => log::error!("Unable to generate aeat 720 form: {:?}", err),
                        }
                    }
                    Err(err) => log::error!("Unable to generate Aeat 720 report: {}", err),
                }
                true
            }
            Msg::UploadedDegiroFile(file) => {
                log::debug!(
                    "pdf file: {} len: {}, content: {:X?}",
                    file.name,
                    file.content.len(),
                    file.content.get(0..16)
                );
                let pdf_data = read_pdf(file.content);
                if let Ok(data) = pdf_data {
                    let parser = DegiroParser::new(data);
                    let pdf_content = parser.parse_pdf_content();
                    if let Ok((balance_notes, account_notes)) = pdf_content {
                        self.degiro_balance_notes = balance_notes;
                        self.degiro_account_notes = account_notes;
                    } else {
                        log::error!(
                            "Error loading degiro account notes: {}",
                            pdf_content.err().unwrap()
                        );
                    }
                } else {
                    log::error!("Unable to read pdf content");
                }
                self.tasks.clear();
                self.link.send_message(Msg::GenerateD6);
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::UploadedIBFile(file) => {
                log::debug!(
                    "zip file: {} len: {}, content: {:X?}",
                    file.name,
                    file.content.len(),
                    file.content.get(0..16)
                );
                let zip_data = read_zip_str(file.content);
                if let Ok(data) = zip_data {
                    if let Ok(parser) = IBParser::new(&data) {
                        let _ = parser.parse_balance_notes();
                        let _ = parser.parse_account_notes();
                    }
                } else {
                    log::error!("Unable to read zip content");
                }
                self.tasks.clear();
                self.link.send_message(Msg::GenerateD6);
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::UploadDegiroFile(file) => {
                let callback = self.link.callback(Msg::UploadedDegiroFile);
                self.tasks
                    .push(ReaderService::read_file(file, callback).unwrap());
                false
            }
            Msg::UploadIBFile(file) => {
                let callback = self.link.callback(Msg::UploadedIBFile);
                self.tasks
                    .push(ReaderService::read_file(file, callback).unwrap());
                false
            }
            Msg::ErrorUploadPdf => {
                log::error!("Error uploading Degiro pdf");
                false
            }
            Msg::ErrorUploadZip => {
                log::error!("Error uploading InteractiveBrokers zip file");
                false
            }
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        log::debug!("Render App");
        html! {
          <>
            {self.greetings()}
            <hr/>
            {self.get_form_file()}
            <hr/>
            <Container wrap=Wrap::Wrap direction=Direction::Row>
              <Item layouts=vec!(ItemLayout::ItM(6), ItemLayout::ItXs(12))>
                {self.get_balance_notes()}
              </Item>
              <Item layouts=vec!(ItemLayout::ItM(6), ItemLayout::ItXs(12))>
                {self.get_account_notes()}
              </Item>
            </Container>
            <hr/>
            <Container wrap=Wrap::Wrap direction=Direction::Row>
              <Item layouts=vec!(ItemLayout::ItM(6), ItemLayout::ItXs(12)) align_self=AlignSelf::Center>
                <center>{self.get_d6_button()}</center>
              </Item>
              <Item layouts=vec!(ItemLayout::ItM(6), ItemLayout::ItXs(12)) align_self=AlignSelf::Center>
                <center>{self.get_aeat720_button()}</center>
              </Item>
            </Container>

          </>
        }
    }
}

impl App {
    fn greetings(&self) -> Html {
        html! {
          <>
            <h2>{"Burocratin te ayuda a rellenar los formularios D6 y 720 a partir de los informes de tus brokers."}</h2>
            <p>
              {"Burocratin utiliza la tecnología "} <a href="https://en.wikipedia.org/wiki/WebAssembly" alt="WebAssembly">{"WebAssembly"}</a>
              {" con lo cual una vez la página realiza la carga inicial toda acción es local y ningún dato viaja por la red."}
            </p>
            <p>
              <a href="mailto:contacto@burocratin.com" alt="contacto">{"Escríbeme"}</a>{" para cualquier duda o sugerencia."}
            </p>
          </>
        }
    }

    fn get_form_file(&self) -> Html {
        html! {
            <Container wrap=Wrap::Wrap direction=Direction::Row>
                <Item layouts=vec!(ItemLayout::ItM(6), ItemLayout::ItXs(12))>
                    <FormGroup orientation=Orientation::Horizontal>
                        <img src="img/degiro.svg" alt="logo broker Degiro" width="70" height="70" />
                        <FormLabel text="Informe anual broker Degiro:" />
                        <FormFile
                            alt="Fichero informe broker Degiro"
                            accept=vec!["application/pdf".to_string()]
                            underline=false
                            onchange_signal = self.link.callback(|data: ChangeData | {
                                if let ChangeData::Files(files) = data {
                                    let file = files.get(0).unwrap();
                                    Msg::UploadDegiroFile(file)
                                } else {
                                    Msg::ErrorUploadPdf
                                }
                            })
                        />
                    </FormGroup>
                </Item>
                <Item layouts=vec!(ItemLayout::ItM(6), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                    <img src="img/interactive_brokers.svg" alt="logo interactive brokers" width="70" height="70" />
                    <FormLabel text="Informe anual Interactive Brokers:" />
                    <FormFile
                        alt="Fichero informe Interactive Brokers"
                        accept=vec!["application/zip".to_string()]
                        underline=false
                        onchange_signal = self.link.callback(|data: ChangeData | {
                            if let ChangeData::Files(files) = data {
                                let file = files.get(0).unwrap();
                                Msg::UploadIBFile(file)
                            } else {
                                Msg::ErrorUploadZip
                            }
                        })
                    />
                </FormGroup>
            </Item>
            </Container>
        }
    }

    fn get_account_notes(&self) -> Html {
        let notes = self
            .degiro_account_notes
            .iter()
            .map(|note| {
                html! {
                <tr>
                  <td>{"Degiro"}</td>
                  <td>{&note.company.name}</td>
                  <td>{&note.company.isin}</td>
                  <td>{&note.value_in_euro}</td>
                </tr>}
            })
            .collect::<Html>();

        html! {
            <table>
            <caption>{"Movimientos brokers"}</caption>
            <thead>
              <tr>
                <th>{"Broker"}</th>
                <th>{"Acción"}</th>
                <th>{"ISIN"}</th>
                <th>{"Valor (€)"}</th>
              </tr>
            </thead>
            <tbody>
            {notes}
            </tbody>
            </table>
        }
    }

    fn get_balance_notes(&self) -> Html {
        let notes = self
            .degiro_balance_notes
            .iter()
            .map(|note| {
                html! {
                <tr>
                  <td>{"Degiro"}</td>
                  <td>{&note.company.name}</td>
                  <td>{&note.company.isin}</td>
                  <td>{&note.value_in_euro}</td>
                </tr>}
            })
            .collect::<Html>();

        html! {
            <table>
            <caption>{"Balance brokers"}</caption>
            <thead>
              <tr>
                <th>{"Broker"}</th>
                <th>{"Acción"}</th>
                <th>{"ISIN"}</th>
                <th>{"Valor (€)"}</th>
              </tr>
            </thead>
            <tbody>
            {notes}
            </tbody>
            </table>
        }
    }

    fn get_d6_button(&self) -> Html {
        if !self.d6_form_path.is_empty() {
            html! {
              <a href={self.d6_form_path.clone()} alt="Informe D6 generado" download="d6.aforixm"><button type={"button"}>{"Descargar informe D6"}</button></a>
            }
        } else {
            html! {
                <button disabled=true type={"button"}>{"Descargar informe D6"}</button>
            }
        }
    }

    fn get_aeat720_button(&self) -> Html {
        if !self.aeat720_form_path.is_empty() {
            html! {
              <a href={self.aeat720_form_path.clone()} alt="Informe D6 generado" download="fichero-720.txt"><button type={"button"}>{"Descargar informe AEAT 720"}</button></a>
            }
        } else {
            html! {
                <button disabled=true type={"button"}>{"Descargar informe AEAT 720"}</button>
            }
        }
    }
}
