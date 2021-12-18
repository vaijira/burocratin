use std::rc::Rc;
use std::vec;

use crate::data::{BrokerInformation, FinancialInformation};
use crate::parsers::degiro_csv::DegiroCSVParser;
use crate::utils::web;
use crate::{parsers::degiro::DegiroParser, parsers::ib::IBParser};
use crate::{parsers::pdf::read_pdf, utils::zip::read_zip_str};

use yew::prelude::*;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew_assets::info_assets::{InfoAssets, InfoIcon};
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
use yew_styles::styles::{Palette, Position, Size, Style};
use yew_styles::text::{Text, TextType};
use yew_styles::tooltip::Tooltip;

const DEFAULT_YEAR: usize = 2021;
const DEFAULT_UX_COLOR: &str = "#FFFFFF";
const DEFAULT_UX_SIZE: &str = "30";

pub struct App {
    degiro_broker: Rc<BrokerInformation>,
    ib_broker: Rc<BrokerInformation>,
    tasks: Vec<ReaderTask>,
    financial_information: FinancialInformation,
    aeat720_form_path: String,
    link: ComponentLink<Self>,
}

pub enum Msg {
    ChangeName(String),
    ChangeSurname(String),
    ChangeNif(String),
    ChangeYear(String),
    GenerateAeat720,
    UploadDegiroFile(File),
    UploadDegiroCSVFile(File),
    UploadIBFile(File),
    UploadedDegiroFile(FileData),
    UploadedDegiroCSVFile(FileData),
    UploadedIBFile(FileData),
    ErrorUploadPdf,
    ErrorUploadCSV,
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
            aeat720_form_path: "".to_string(),
            link,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        match message {
            Msg::ChangeName(name) => {
                log::debug!("change name to: {}", name);
                self.financial_information.name = name.to_uppercase();
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::ChangeSurname(surname) => {
                log::debug!("change surname to: {}", surname);
                self.financial_information.surname = surname.to_uppercase();
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::ChangeNif(nif) => {
                log::debug!("change nif to: {}", nif);
                self.financial_information.nif = nif.to_uppercase();
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::ChangeYear(year) => {
                log::debug!("change year to: {}", year);
                self.financial_information.year = year.parse::<usize>().unwrap_or(DEFAULT_YEAR);
                self.link.send_message(Msg::GenerateAeat720);
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
                            "Error loading degiro pdf notes: {}",
                            pdf_content.err().unwrap()
                        );
                    }
                } else {
                    log::error!("Unable to read pdf content");
                }
                self.tasks.clear();
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::UploadedDegiroCSVFile(file) => {
                log::debug!(
                    "pdf file: {} len: {}, content: {:X?}",
                    file.name,
                    file.content.len(),
                    file.content.get(0..16)
                );

                if let Ok(data) = String::from_utf8(file.content) {
                    let parser = DegiroCSVParser::new(data, &self.degiro_broker);
                    let csv_content = parser.parse_csv();
                    if let Ok(mut balance_notes) = csv_content {
                        self.financial_information
                            .account_notes
                            .retain(|note| note.broker != self.degiro_broker);
                        self.financial_information
                            .balance_notes
                            .retain(|note| note.broker != self.degiro_broker);
                        self.financial_information
                            .balance_notes
                            .append(&mut balance_notes);
                    } else {
                        log::error!(
                            "Error loading degiro csv notes: {}",
                            csv_content.err().unwrap()
                        );
                    }
                } else {
                    log::error!("Unable to read csv content");
                }
                self.tasks.clear();
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
                self.link.send_message(Msg::GenerateAeat720);
                true
            }
            Msg::UploadDegiroFile(file) => {
                let callback = self.link.callback(Msg::UploadedDegiroFile);
                self.tasks
                    .push(ReaderService::read_file(file, callback).unwrap());
                false
            }
            Msg::UploadDegiroCSVFile(file) => {
                let callback = self.link.callback(Msg::UploadedDegiroCSVFile);
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
            Msg::ErrorUploadCSV => {
                log::error!("Error uploading Degiro CSV");
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
              <Item layouts=vec!(ItemLayout::ItM(12), ItemLayout::ItXs(12)) align_self=AlignSelf::Center>
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
            {"A efectos prácticos (a menos que controles más del 10% de una empresa) el D-6 ha quedado "}<a href="https://www.boe.es/boe/dias/2021/12/17/pdfs/BOE-A-2021-20816.pdf" alt="cambio condiciones D-6">{"obsoleto"}</a>{" por lo que se desactiva."}
            </p>
            <p>
              {"Para cualquier mejora, duda, sugerencia o error puedes crear un "}
              <a href="https://github.com/vaijira/burocratin/issues" alt="github issue">{"ticket"}</a>
              {" o mandar un "}<a href="mailto:contacto@burocratin.com" alt="contacto">{"mail"}</a>{"."}
            </p>
            <p>
              {"El modelo 720 generado se puede presentar si es la primera declaración o "}
              <a href="https://www.agenciatributaria.es/AEAT.internet/Inicio/Ayuda/Modelos__Procedimientos_y_Servicios/Ayuda_Modelo_720/Informacion_general/Preguntas_frecuentes__actualizadas_a_marzo_de_2014_/Nuevas_preguntas_frecuentes/Si_se_procede_a_la_venta_de_valores__articulo_42_ter_del_Reglamento_General_aprobado_por_el_RD_1065_2007___respecto_de_los_qu__on_de_informar_.shtml" alt="720 FAQ">
              {"si se ha realizado alguna venta y reinvertido el importe"}</a>{"."}
            </p>
            <ul>{"Limitaciones:"}
            <li>{"Sólo rellena información de acciones, no líquidez del broker."}</li>
            <li>{"El código de país que usará para Degiro será NL y para interactive brokers IE."}</li>
            <li>{"Modelo 720:Revisar el código de domiciliación del país, por defecto cogerá el del ISIN, pero esto no siempre es correcto."}</li>
            <li>{"Modelo 720: Revisar la fecha de primera incorporación si tu primera transacción fue antes del año a declarar."}</li>
            </ul>
            <p>{"El autor no se hace responsable del uso resultante de esta aplicación."}</p>
          </>
        }
    }

    fn help_degiro_pdf(&self) -> Html {
        html! {
            <ul>
                {"Para descargar el informe anual de degiro en pdf: "}
                <li>
                {"Entre en la página de degiro con su usuario."}
                </li>
                <li>
                {"En el menú izquierdo pulse Actividad y seguidamente informes."}
                </li>
                <li>
                {"En informes seleccione el informe anual del año a declarar y  pulse descargar."}
                </li>
            </ul>
        }
    }

    fn help_degiro_csv(&self) -> Html {
        html! {
            <ul>
                {"Para descargar las posiciones de degiro en csv: "}
                <li>
                {"Entre en la página de degiro con su usuario."}
                </li>
                <li>
                {"En el menú izquierdo pulse Cartera."}
                </li>
                <li>
                {"Arriba a la derecha pulse el botón exportar."}
                </li>
                <li>
                {"Seleccione la fecha de 31 de Diciembre del año a declarar y pulse CSV."}
                </li>
            </ul>
        }
    }

    fn help_ib_report(&self) -> Html {
        html! {
            <ul>
                {"Para descargar el informe anual de interactive brokers: "}
                <li>
                {"Entre en la página de interactive brokers con su usuario."}
                </li>
                <li>
                {"En el menú superior seleccione informes y seguidamente extractos."}
                </li>
                <li>
                {"Si ha tenido más de 1 cuenta seleccione todas pulsando en el identificador de usuario al lado de informes y seleccionando todos."}
                </li>
                <li>
                {"En extractos predeterminados pulse en actividad, seleccione el período anual, el formato HTML/descargar y en opciones zip."}
                </li>
                <li>
                {"Pulse ejecutar."}
                </li>
            </ul>
        }
    }

    fn get_form_file(&self) -> Html {
        html! {
            <Container wrap=Wrap::Wrap direction=Direction::Row>
                <Item layouts=vec!(ItemLayout::ItM(4), ItemLayout::ItXs(12))>

                    <FormGroup orientation=Orientation::Horizontal>
                    <img src="img/degiro.svg" alt="logo broker Degiro" width="70" height="70" />
                        <FormLabel text="Informe anual broker Degiro (PDF):" />
                        <FormFile
                            id={"degiro_report"}
                            alt="Fichero PDF informe broker Degiro"
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
                        <Tooltip
                          tooltip_palette=Palette::Clean
                          tooltip_style=Style::Outline
                          tooltip_position=Position::Above
                          tooltip_size=Size::Medium
                          content=self.help_degiro_pdf()
                          class_name="tooltip-page">

                          <div class="tooltip-content">
                            <InfoAssets
                              icon=InfoIcon::HelpCircle
                              fill=DEFAULT_UX_COLOR
                              size=(DEFAULT_UX_SIZE.to_string(), DEFAULT_UX_SIZE.to_string())/></div>
                        </Tooltip>
                    </FormGroup>
                </Item>
                <Item layouts=vec!(ItemLayout::ItM(4), ItemLayout::ItXs(12))>
                    <FormGroup orientation=Orientation::Horizontal>
                    <img src="img/degiro.svg" alt="logo broker Degiro" width="70" height="70" />
                    <FormLabel text="Informe anual broker Degiro (CSV):" />
                    <FormFile
                        id={"degiro_csv_report"}
                        alt="Fichero CSV informe broker Degiro"
                        accept=vec!["text/csv".to_string()]
                        underline=false
                        onchange_signal = self.link.callback(|data: ChangeData | {
                            if let ChangeData::Files(files) = data {
                                let file = files.get(0).unwrap();
                                Msg::UploadDegiroCSVFile(file)
                            } else {
                                Msg::ErrorUploadCSV
                            }
                        })
                    />
                    <Tooltip
                    tooltip_palette=Palette::Clean
                    tooltip_style=Style::Outline
                    tooltip_position=Position::Above
                    tooltip_size=Size::Medium
                    content=self.help_degiro_csv()
                    class_name="tooltip-page">

                    <div class="tooltip-content">
                      <InfoAssets
                        icon=InfoIcon::HelpCircle
                        fill=DEFAULT_UX_COLOR
                        size=(DEFAULT_UX_SIZE.to_string(), DEFAULT_UX_SIZE.to_string())/></div>
                    </Tooltip>
                </FormGroup>
                </Item>
                <Item layouts=vec!(ItemLayout::ItM(4), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                    <img src="img/interactive_brokers.svg" alt="logo interactive brokers" width="70" height="70" />
                    <FormLabel text="Informe anual Interactive Brokers (ZIP):" />
                    <FormFile
                        id={"ib_report"}
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
                    <Tooltip
                    tooltip_palette=Palette::Clean
                    tooltip_style=Style::Outline
                    tooltip_position=Position::Left
                    tooltip_size=Size::Medium
                    content=self.help_ib_report()
                    class_name="tooltip-page">

                    <div class="tooltip-content">
                      <InfoAssets
                        icon=InfoIcon::HelpCircle
                        fill=DEFAULT_UX_COLOR
                        size=(DEFAULT_UX_SIZE.to_string(), DEFAULT_UX_SIZE.to_string())/></div>
                    </Tooltip>
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
                  id={"name"}
                  alt={"Nombre"}
                  input_type=InputType::Text
                  oninput_signal=self.link.callback(|e: InputData| Msg::ChangeName(e.value))
                />
                </FormGroup>
              </Item>

              <Item layouts=vec!(ItemLayout::ItM(3), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                <FormLabel text="Apellidos: " />
                <FormInput
                  id={"surname"}
                  alt={"Apellidos"}
                  input_type=InputType::Text
                  oninput_signal=self.link.callback(|e: InputData| Msg::ChangeSurname(e.value))
                />
                </FormGroup>
              </Item>

              <Item layouts=vec!(ItemLayout::ItM(3), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                <FormLabel text="NIF: " />
                <FormInput
                  id={"nif"}
                  alt={"NIF"}
                  input_type=InputType::Text
                  oninput_signal=self.link.callback(|e: InputData| Msg::ChangeNif(e.value))
                />
                </FormGroup>
              </Item>

              <Item layouts=vec!(ItemLayout::ItM(3), ItemLayout::ItXs(12))>
                <FormGroup orientation=Orientation::Horizontal>
                <FormLabel text="Año: " />
                <FormInput
                  id={"year"}
                  alt={"Año"}
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

    fn get_aeat720_button(&self) -> Html {
        if !self.aeat720_form_path.is_empty() {
            html! {
              <a id={"aeat_720_form"} href={self.aeat720_form_path.clone()} alt="Informe D6 generado" download="fichero-720.txt"><button type={"button"}>{"Descargar informe AEAT 720"}</button></a>
            }
        } else {
            html! {
                <button disabled=true type={"button"}>{"Descargar informe AEAT 720"}</button>
            }
        }
    }
}
