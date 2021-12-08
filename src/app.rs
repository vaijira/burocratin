use std::rc::Rc;
use std::vec;

use crate::data::{BrokerInformation, FinancialInformation};
use crate::utils::web;
use crate::{parsers::degiro::DegiroParser, parsers::ib::IBParser};
use crate::{parsers::pdf::read_pdf, utils::zip::read_zip_str};

use yew::prelude::*;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew_styles::forms::{
    form_file::FormFile,
    form_group::{FormGroup, Orientation},
    form_input::{FormInput, InputType},
    form_label::FormLabel,
};
use yew_styles::layouts::{
    container::{Container, Direction, Wrap},
    item::{AlignSelf, Item, ItemLayout},
};
use yew_styles::styles::Size;
use yew_styles::text::{Text, TextType};

const DEFAULT_YEAR: usize = 2020;

pub struct App {
    degiro_broker: Rc<BrokerInformation>,
    ib_broker: Rc<BrokerInformation>,
    tasks: Vec<ReaderTask>,
    financial_information: FinancialInformation,
    d6_form_path: String,
    aeat720_form_path: String,
    link: ComponentLink<Self>,
}

pub enum Msg {
    ChangeName(String),
    ChangeSurname(String),
    ChangeNif(String),
    ChangeYear(String),
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
        let mut info = FinancialInformation::new();
        info.year = DEFAULT_YEAR;
        Self {
            degiro_broker: Rc::new(BrokerInformation::new(
                String::from("Degiro"),
                String::from("NL"),
            )),
            ib_broker: Rc::new(BrokerInformation::new(
                String::from("Interactive Brokers"),
                String::from("IE"),
            )),
            tasks: vec![],
            financial_information: info,
            d6_form_path: "".to_string(),
            aeat720_form_path: "".to_string(),
            link,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        match message {
            Msg::ChangeName(name) => {
                log::debug!("change name to: {}", name);
                self.financial_information.name = name;
                self.link.send_message(Msg::GenerateD6);
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::ChangeSurname(surname) => {
                log::debug!("change surname to: {}", surname);
                self.financial_information.surname = surname;
                self.link.send_message(Msg::GenerateD6);
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::ChangeNif(nif) => {
                log::debug!("change nif to: {}", nif);
                self.financial_information.nif = nif;
                self.link.send_message(Msg::GenerateD6);
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::ChangeYear(year) => {
                log::debug!("change year to: {}", year);
                self.financial_information.year = year.parse::<usize>().unwrap_or(DEFAULT_YEAR);
                self.link.send_message(Msg::GenerateD6);
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::GenerateD6 => {
                log::debug!("generate D6 form");
                if let Ok(path) = web::generate_d6(
                    &self.financial_information.balance_notes,
                    &self.d6_form_path,
                ) {
                    self.d6_form_path = path;
                }
                true
            }
            Msg::GenerateAeat720 => {
                log::debug!("generate AEAT 720 form");
                if let Ok(path) =
                    web::generate_720(&self.financial_information, &self.aeat720_form_path)
                {
                    self.aeat720_form_path = path;
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

                if let Ok(data) = read_pdf(file.content) {
                    let parser = DegiroParser::new(data, &self.degiro_broker);
                    let pdf_content = parser.parse_pdf_content();
                    if let Ok((mut balance_notes, mut account_notes)) = pdf_content {
                        self.financial_information
                            .account_notes
                            .retain(|note| note.broker != self.degiro_broker);
                        self.financial_information
                            .balance_notes
                            .retain(|note| note.broker != self.degiro_broker);
                        self.financial_information
                            .account_notes
                            .append(&mut account_notes);
                        self.financial_information
                            .balance_notes
                            .append(&mut balance_notes);
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

                if let Ok(data) = read_zip_str(file.content) {
                    if let Ok(parser) = IBParser::new(&data, &self.ib_broker) {
                        let account_notes = parser.parse_account_notes();
                        let balance_notes = parser.parse_balance_notes();
                        if let (Ok(mut account_notes), Ok(mut balance_notes)) =
                            (account_notes, balance_notes)
                        {
                            self.financial_information
                                .account_notes
                                .retain(|note| note.broker != self.ib_broker);
                            self.financial_information
                                .balance_notes
                                .retain(|note| note.broker != self.ib_broker);
                            self.financial_information
                                .account_notes
                                .append(&mut account_notes);
                            self.financial_information
                                .balance_notes
                                .append(&mut balance_notes);
                        } else {
                            log::error!("Unable to read interactive brokers info");
                        }
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
            {self.get_personal_information()}
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
            <p>
              {"El modelo 720 generado se puede presentar si es la primera declaración o "}
              <a href="https://www.agenciatributaria.es/AEAT.internet/Inicio/Ayuda/Modelos__Procedimientos_y_Servicios/Ayuda_Modelo_720/Informacion_general/Preguntas_frecuentes__actualizadas_a_marzo_de_2014_/Nuevas_preguntas_frecuentes/Si_se_procede_a_la_venta_de_valores__articulo_42_ter_del_Reglamento_General_aprobado_por_el_RD_1065_2007___respecto_de_los_qu__on_de_informar_.shtml" alt="720 FAQ">
              {"si se ha realizado alguna venta y reinvertido el importe"}</a>{"."}
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
                    <FormLabel text="Informe anual Interactive Brokers (zip):" />
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

    fn get_personal_information(&self) -> Html {
        html! {
            <>
             <Text
              text_type=TextType::Plain
              text_size=Size::Medium
              plain_text="Rellena los siguientes campos si quieres que los informes se generen con ellos:"
              html_text=None
            />
            <Container wrap=Wrap::Wrap direction=Direction::Row>

              <Item layouts=vec!(ItemLayout::ItM(3), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                <FormLabel text="Nombre: " />
                <FormInput
                  input_type=InputType::Text
                  oninput_signal=self.link.callback(|e: InputData| Msg::ChangeName(e.value))
                />
                </FormGroup>
              </Item>

              <Item layouts=vec!(ItemLayout::ItM(3), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                <FormLabel text="Apellidos: " />
                <FormInput
                  input_type=InputType::Text
                  oninput_signal=self.link.callback(|e: InputData| Msg::ChangeSurname(e.value))
                />
                </FormGroup>
              </Item>

              <Item layouts=vec!(ItemLayout::ItM(3), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                <FormLabel text="NIF: " />
                <FormInput
                  input_type=InputType::Text
                  oninput_signal=self.link.callback(|e: InputData| Msg::ChangeNif(e.value))
                />
                </FormGroup>
              </Item>

              <Item layouts=vec!(ItemLayout::ItM(3), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                <FormLabel text="Año: " />
                <FormInput
                  placeholder=self.financial_information.year.to_string()
                  input_type=InputType::Text
                  oninput_signal=self.link.callback(|e: InputData| Msg::ChangeYear(e.value))
                />
                </FormGroup>
              </Item>

            </Container>
            </>
        }
    }

    fn get_account_notes(&self) -> Html {
        let notes = self
            .financial_information
            .account_notes
            .iter()
            .map(|note| {
                html! {
                <tr>
                  <td>{&note.broker.name}</td>
                  <td>{&note.company.name}</td>
                  <td>{&note.company.isin}</td>
                  <td>{&note.value}</td>
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
            .financial_information
            .balance_notes
            .iter()
            .map(|note| {
                html! {
                <tr>
                  <td>{&note.broker.name}</td>
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
