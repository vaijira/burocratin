use pdf::content::*;
use pdf::encoding::BaseEncoding;
use pdf::error::PdfError;
use pdf::file::File as PdfFile;
use pdf::font::*;
use pdf::object::NoResolve;
use pdf::parser::parse_with_lexer;
use pdf::parser::Lexer;
use pdf::primitive::Primitive;
use std::collections::HashMap;
use std::convert::TryInto;

use byteorder::BE;
use utf16_ext::Utf16ReadExt;

pub fn read_pdf(data: Vec<u8>) -> Result<String, PdfError> {
    let file = PdfFile::<Vec<u8>>::from_data(data).unwrap();
    let mut out = String::new();
    for page in file.pages() {
        let resources = page.as_ref().unwrap().resources(&file).unwrap();
        let mut cache = FontCache::new();

        // make sure all fonts are in the cache, so we can reference them
        for (name, font) in &resources.fonts {
            cache.add_font(name, font);
        }
        for gs in resources.graphics_states.values() {
            if let Some((ref font, _)) = gs.font {
                cache.add_font(font.name.as_str(), font);
            }
        }
        let mut current_font = None;
        let page = page.unwrap();
        let contents = page.contents.as_ref().unwrap();
        for Operation {
            ref operator,
            ref operands,
        } in &contents.operations
        {
            log::trace!("{} {:?}", operator, operands);
            match operator.as_str() {
                "gs" => {
                    let gs = resources
                        .graphics_states
                        .get(operands[0].as_name().unwrap())
                        .unwrap();

                    if let Some((ref font, _)) = gs.font {
                        current_font = cache.get_font(&font.name);
                    }
                }
                // text font
                "Tf" => {
                    let font_name = operands[0].as_name().expect("font name is not a string");
                    log::trace!("font name: {}", font_name);
                    current_font = cache.get_font(font_name);
                }
                "Tj" | "TJ" | "BT" => {
                    if let Some(font) = current_font {
                        operands
                            .iter()
                            .for_each(|p| add_primitive(p, &mut out, font));
                    }
                }
                "Td" | "TD" | "T*" => {
                    out.push('\n');
                }
                _ => {}
            }
        }
    }

    log::debug!("{}", out);

    Ok(out)
}

fn utf16be_to_string(mut data: &[u8]) -> String {
    (&mut data)
        .utf16_chars::<BE>()
        .map(|c| c.unwrap())
        .collect()
}

// totally not a steaming pile of hacks
fn parse_cmap(data: &[u8]) -> HashMap<u16, String> {
    log::trace!("{}", std::str::from_utf8(data).unwrap());
    let mut lexer = Lexer::new(data);
    let mut map = HashMap::new();
    while let Ok(substr) = lexer.next() {
        match substr.as_slice() {
            b"beginbfchar" => loop {
                let a = parse_with_lexer(&mut lexer, &NoResolve);
                let b = parse_with_lexer(&mut lexer, &NoResolve);
                match (a, b) {
                    (Ok(Primitive::String(cid_data)), Ok(Primitive::String(unicode_data))) => {
                        let cid = u16::from_be_bytes(cid_data.as_bytes().try_into().unwrap());
                        let unicode = utf16be_to_string(unicode_data.as_bytes());
                        map.insert(cid, unicode);
                    }
                    _ => break,
                }
            },
            b"beginbfrange" => loop {
                let a = parse_with_lexer(&mut lexer, &NoResolve);
                let b = parse_with_lexer(&mut lexer, &NoResolve);
                let c = parse_with_lexer(&mut lexer, &NoResolve);
                match (a, b, c) {
                    (
                        Ok(Primitive::String(cid_start_data)),
                        Ok(Primitive::String(cid_end_data)),
                        Ok(Primitive::String(unicode_data)),
                    ) => {
                        let cid_start =
                            u16::from_be_bytes(cid_start_data.as_bytes().try_into().unwrap());
                        let cid_end =
                            u16::from_be_bytes(cid_end_data.as_bytes().try_into().unwrap());
                        let mut unicode_data = unicode_data.into_bytes();

                        for cid in cid_start..=cid_end {
                            let unicode = utf16be_to_string(&unicode_data);
                            map.insert(cid, unicode);
                            *unicode_data.last_mut().unwrap() += 1;
                        }
                    }
                    (
                        Ok(Primitive::String(cid_start_data)),
                        Ok(Primitive::String(cid_end_data)),
                        Ok(Primitive::Array(unicode_data_arr)),
                    ) => {
                        let cid_start =
                            u16::from_be_bytes(cid_start_data.as_bytes().try_into().unwrap());
                        let cid_end =
                            u16::from_be_bytes(cid_end_data.as_bytes().try_into().unwrap());

                        for (cid, unicode_data) in (cid_start..=cid_end).zip(unicode_data_arr) {
                            let unicode =
                                utf16be_to_string(&unicode_data.as_string().unwrap().as_bytes());
                            map.insert(cid, unicode);
                        }
                    }
                    _ => break,
                }
            },
            b"endcmap" => break,
            _ => {}
        }
    }

    map
}

struct FontInfo<'a> {
    font: &'a Font,
    cmap: HashMap<u16, String>,
}

struct FontCache<'a> {
    fonts: HashMap<&'a str, FontInfo<'a>>,
}

impl<'a> FontCache<'a> {
    fn new() -> Self {
        FontCache {
            fonts: HashMap::new(),
        }
    }
    fn add_font(&mut self, name: &'a str, font: &'a Font) {
        if let Some(to_unicode) = font.to_unicode() {
            let cmap = parse_cmap(to_unicode.data().unwrap());
            self.fonts.insert(name, FontInfo { font, cmap });
        }
    }
    fn get_font<'b>(&self, name: &'b str) -> Option<&FontInfo<'a>> {
        self.fonts.get(&*name)
    }
}

fn add_primitive(p: &Primitive, out: &mut String, info: &FontInfo<'_>) {
    log::trace!("p: {:?}", p);
    match *p {
        Primitive::String(ref data) => {
            if let Some(encoding) = info.font.encoding() {
                match encoding.base {
                    BaseEncoding::IdentityH => {
                        for w in data.as_bytes().windows(2) {
                            let cp = u16::from_be_bytes(w.try_into().unwrap());
                            if let Some(s) = info.cmap.get(&cp) {
                                out.push_str(s);
                            }
                        }
                        out.push('\n');
                    }
                    _ => {
                        for &b in data.as_bytes() {
                            if let Some(s) = info.cmap.get(&(b as u16)) {
                                out.push_str(s);
                            } else {
                                out.push(b as char);
                            }
                        }
                    }
                };
            }
        }
        Primitive::Array(ref a) => {
            for p in a.iter() {
                add_primitive(p, out, info);
            }
        }
        _ => (),
    }
}
