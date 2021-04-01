use std::vec;

use crate::{account_notes::AccountNotes, degiro_parser::DegiroParser};
use crate::{account_notes::BalanceNotes, pdf_parser::read_pdf};

use yew::prelude::*;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew_styles::forms::{
    form_file::FormFile,
    form_group::{FormGroup, Orientation},
    form_label::FormLabel,
};
use yew_styles::layouts::{
    container::{Container, Direction, Wrap},
    item::{Item, ItemLayout},
};

pub struct App {
    reader: ReaderService,
    tasks: Vec<ReaderTask>,
    degiro_balance_notes: BalanceNotes,
    degiro_account_notes: AccountNotes,
    link: ComponentLink<Self>,
}

pub enum Msg {
    UploadFile(File),
    UploadedFile(FileData),
    ErrorUploadPdf,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        log::debug!("App created");
        Self {
            reader: ReaderService::new(),
            tasks: vec![],
            degiro_balance_notes: vec![],
            degiro_account_notes: vec![],
            link,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        match message {
            Msg::UploadedFile(file) => {
                log::debug!(
                    "file: {} len: {}, content: {:X?}",
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
                true
            }
            Msg::UploadFile(file) => {
                let callback = self.link.callback(Msg::UploadedFile);
                self.tasks
                    .push(self.reader.read_file(file, callback).unwrap());
                false
            }
            Msg::ErrorUploadPdf => {
                log::error!("Error to upload pdf");
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
            {self.get_form_file()}
            {self.get_balance_notes()}
            {self.get_account_notes()}
          </>
        }
    }
}

impl App {
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
                                    Msg::UploadFile(file)
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
                        accept=vec!["application/xhtml+xml".to_string(), "text/html".to_string()]
                        underline=false
                        onchange_signal = self.link.callback(|data: ChangeData | {
                            if let ChangeData::Files(files) = data {
                                let file = files.get(0).unwrap();
                                Msg::UploadFile(file)
                            } else {
                                Msg::ErrorUploadPdf
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
            <caption>{"Movimientos broker Degiro"}</caption>
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
            <caption>{"Balance broker Degiro"}</caption>
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
}
