use std::io::IsTerminal;

use polars::prelude::*;

use crate::commands::report::data::PerMetricSummary;

pub fn print_md(s: impl AsRef<str>) {
    let mut printer = MarkdownPrinter::new();
    printer.add(s);
    printer.dump();
}

pub struct MarkdownPrinter {
    content: String,
}

impl MarkdownPrinter {
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn is_tty(&self) -> bool {
        std::io::stdout().is_terminal()
    }

    pub fn dump(&self) {
        if self.is_tty() {
            let mut skin = termimad::MadSkin::default();
            for i in 0..8 {
                skin.headers[i].align = termimad::Alignment::Left;
                skin.headers[i].add_attr(termimad::crossterm::style::Attribute::Bold);
                skin.headers[i].set_fg(termimad::crossterm::style::Color::Blue);
            }
            skin.headers[0].set_bg(termimad::crossterm::style::Color::Blue);
            skin.headers[0].add_attr(termimad::crossterm::style::Attribute::NoUnderline);
            skin.print_text(&self.content);
        } else {
            println!("{}", self.content);
        }
    }

    pub fn add(&mut self, s: impl AsRef<str>) {
        self.content.push_str(s.as_ref());
    }

    pub fn add_dataframe_with_ci(&mut self, df: &DataFrame, ci: &DataFrame) {
        let md_table = self.df_to_markdown(df, Some(ci));
        self.content.push_str(&md_table);
    }

    pub fn add_metric_summary(&mut self, s: &PerMetricSummary) {
        let md_table = self.metric_summary_to_markdown(s);
        self.content.push_str(&md_table);
    }

    fn any_value_to_table_cell(&self, v: &AnyValue) -> TableCell {
        match v {
            AnyValue::Float32(v) => TableCell::Float(*v as f64),
            AnyValue::Float64(v) => TableCell::Float(*v),
            AnyValue::Int8(v) => TableCell::Int(*v as i64),
            AnyValue::Int16(v) => TableCell::Int(*v as i64),
            AnyValue::Int32(v) => TableCell::Int(*v as i64),
            AnyValue::Int64(v) => TableCell::Int(*v as i64),
            AnyValue::UInt8(v) => TableCell::Int(*v as i64),
            AnyValue::UInt16(v) => TableCell::Int(*v as i64),
            AnyValue::UInt32(v) => TableCell::Int(*v as i64),
            AnyValue::UInt64(v) => TableCell::Int(*v as i64),
            v if v.get_str().is_some() => TableCell::Label(v.get_str().unwrap().to_string()),
            _ => unimplemented!("{:?}", v),
        }
    }

    fn get_f64(&self, v: &AnyValue) -> f64 {
        match v {
            AnyValue::Float32(v) => *v as f64,
            AnyValue::Float64(v) => *v,
            _ => unimplemented!(),
        }
    }

    fn df_to_markdown(&self, df: &DataFrame, ci: Option<&DataFrame>) -> String {
        let mut table = MarkdownTable::default();
        for col in df.get_columns() {
            let name = col.name();
            table.headers.push(name.to_owned());
            // Initialize rows
            if table.rows.is_empty() {
                table.rows = (0..col.len()).map(|_| vec![]).collect::<Vec<_>>();
            }
            // Collect cells for this column
            if let Some(ci) = ci {
                for i in 0..col.len() {
                    if let Some(ci_col) = ci
                        .column(name)
                        .iter()
                        .find(|ci_col| col.dtype().is_numeric() && ci_col.name() == col.name())
                    {
                        let v = self.get_f64(&col.get(i).unwrap());
                        let ci = self.get_f64(&ci_col.get(i).unwrap());
                        table.rows[i].push(TableCell::FloatWithCI(v, ci));
                    } else {
                        let cell = self.any_value_to_table_cell(&col.get(i).unwrap());
                        table.rows[i].push(cell);
                    }
                }
            } else {
                for i in 0..col.len() {
                    let cell = self.any_value_to_table_cell(&col.get(i).unwrap());
                    table.rows[i].push(cell);
                }
            }
        }
        table.render()
    }

    fn metric_summary_to_markdown(&self, s: &PerMetricSummary) -> String {
        let mut table = MarkdownTable::default();
        for col in s.unnormed.get_columns() {
            let name = col.name();
            table.headers.push(name.to_owned());
            let normed_col = s.normed.as_ref().and_then(|df| df.column(name).ok());
            // Initialize rows
            if table.rows.is_empty() {
                table.rows = (0..col.len()).map(|_| vec![]).collect::<Vec<_>>();
            }
            // Collect cells for this column
            if name == "min" || name == "max" {
                for i in 0..col.len() {
                    let v = self.get_f64(&col.get(i).unwrap());
                    let label = if name == "min" {
                        s.min_names[i].clone()
                    } else {
                        s.max_names[i].clone()
                    };
                    if let Some(normed_col) = normed_col {
                        let n = self.get_f64(&normed_col.get(i).unwrap());
                        table.rows[i].push(TableCell::FloatWithNormAndLabel(v, n, label));
                    } else {
                        table.rows[i].push(TableCell::FloatWithLabel(v, label));
                    }
                }
            } else if name == "build" || name == "benchmarks" {
                for i in 0..col.len() {
                    let cell = self.any_value_to_table_cell(&col.get(i).unwrap());
                    table.rows[i].push(cell);
                }
            } else {
                for i in 0..col.len() {
                    let v = self.get_f64(&col.get(i).unwrap());
                    if let Some(normed_col) = normed_col {
                        let n = self.get_f64(&normed_col.get(i).unwrap());
                        table.rows[i].push(TableCell::FloatWithNorm(v, n));
                    } else {
                        table.rows[i].push(TableCell::Float(v));
                    }
                }
            }
        }
        table.render()
    }
}

#[macro_export]
macro_rules! print_md {
    ($($arg:tt)*) => {
        $crate::utils::md::print_md(format!($($arg)*));
    };
}

#[derive(Default)]
struct MarkdownTable {
    headers: Vec<String>,
    rows: Vec<Vec<TableCell>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Right,
}

#[derive(Clone, PartialEq)]
enum TableCell {
    Label(String),
    Float(f64),
    Int(i64),
    FloatWithCI(f64, f64),
    FloatWithNorm(f64, f64),
    FloatWithLabel(f64, String),
    FloatWithNormAndLabel(f64, f64, String),
}

fn pad(c: char, count: usize) -> String {
    (0..count).map(|_| c).collect::<String>()
}

fn pad_end(s: &str, width: usize, c: char) -> String {
    format!("{}{}", s, pad(c, width - s.len()))
}

impl TableCell {
    fn to_str(&self) -> String {
        match self {
            TableCell::Label(s) => s.clone(),
            TableCell::Float(f) => format!("{:.3}", f),
            TableCell::Int(i) => i.to_string(),
            TableCell::FloatWithCI(f, ci) => format!("{:.3} ± {:.3}", f, ci),
            TableCell::FloatWithNorm(f, norm) => format!("{:.3} ({:.3})", f, norm),
            TableCell::FloatWithLabel(f, label) => format!("{:.3} ({})", f, label),
            TableCell::FloatWithNormAndLabel(f, norm, label) => {
                format!("{:.3} ({:.3}, {})", f, norm, label)
            }
        }
    }

    fn get_align(&self) -> Align {
        match self {
            TableCell::Label(_) => Align::Left,
            TableCell::Float(_) => Align::Right,
            TableCell::Int(_) => Align::Right,
            TableCell::FloatWithCI(_, _) => Align::Right,
            TableCell::FloatWithNorm(_, _) => Align::Right,
            TableCell::FloatWithLabel(_, _) => Align::Right,
            TableCell::FloatWithNormAndLabel(_, _, _) => Align::Right,
        }
    }
}
impl MarkdownTable {
    fn lower_to_text_table(&self) -> TextTable {
        let headers = self.headers.clone();
        let mut aligns = vec![];
        if self.rows.is_empty() {
            aligns = vec![Align::Right; headers.len()];
        } else {
            for cell in &self.rows[0] {
                aligns.push(cell.get_align());
            }
        }
        let mut rows = vec![];
        for row in &self.rows {
            let mut r = vec![];
            for cell in row {
                r.push(cell.to_str());
            }
            rows.push(r);
        }
        TextTable {
            headers,
            aligns,
            rows,
            tty: std::io::stdout().is_terminal(),
        }
    }

    fn render(&self) -> String {
        self.lower_to_text_table().render()
    }
}

struct TextTable {
    headers: Vec<String>,
    aligns: Vec<Align>,
    rows: Vec<Vec<String>>,
    tty: bool,
}

impl TextTable {
    fn get_column_widths(&self) -> Vec<usize> {
        let mut col_widths = vec![];
        for i in 0..self.headers.len() {
            let max_width = self
                .rows
                .iter()
                .map(|r| r[i].len())
                .chain(std::iter::once(self.headers[i].len()))
                .max()
                .unwrap();
            col_widths.push(max_width);
        }
        col_widths
    }

    fn render(&self) -> String {
        let col_widths = self.get_column_widths();
        let mut rows = vec![];
        // First row
        if self.tty {
            let cells = col_widths.iter().map(|w| pad('-', *w)).collect::<Vec<_>>();
            let top_row = format!("| {} |", cells.join(" | "));
            rows.push(top_row);
        }
        // Header
        let header = self
            .headers
            .iter()
            .zip(col_widths.iter())
            .map(|(cell, width)| pad_end(cell, *width, ' '))
            .collect::<Vec<_>>()
            .join(" | ");
        let header = format!("| {} |", header);
        rows.push(header);
        // Mid separator with alignment indicators
        let mid = col_widths
            .iter()
            .map(|w| pad('-', *w))
            .enumerate()
            .map(|(i, c)| {
                let left_align = [' ', ':'][(self.aligns[i] == Align::Left) as usize];
                let right_align = [' ', ':'][(self.aligns[i] == Align::Right) as usize];
                format!("{}{}{}", left_align, c, right_align)
            })
            .collect::<Vec<_>>()
            .join(" | ");
        let mid = format!("| {} |", mid);
        rows.push(mid);
        // Value rows
        for row in &self.rows {
            let mid = row
                .iter()
                .zip(col_widths.iter())
                .map(|(cell, width)| pad_end(cell, *width, ' '))
                .collect::<Vec<_>>()
                .join(" | ");
            let mid = format!("| {} |", mid);
            rows.push(mid);
        }
        // Buttom row
        if self.tty {
            let cells = col_widths.iter().map(|w| pad('-', *w)).collect::<Vec<_>>();
            let bottom_row = format!("| {} |", cells.join(" | "));
            rows.push(bottom_row);
        }
        // Concat rows
        let table = rows.join("\n") + "\n";
        table
    }
}
