use serde::Serialize;

#[derive(Serialize)]
#[serde(rename = "$unflatten=legend")]
pub struct XportLegend {
    #[serde(rename = "$unflatten=entry")]
    pub entries: Vec<Box<str>>,
}

#[derive(Serialize)]
#[serde(rename = "$unflatten=meta")]
pub struct XportMetadata {
    #[serde(rename = "$unflatten=start")]
    pub start: u64,
    #[serde(rename = "$unflatten=step")]
    pub step: u64,
    #[serde(rename = "$unflatten=end")]
    pub end: u64,
    #[serde(rename = "$unflatten=rows")]
    pub rows: u64,
    #[serde(rename = "$unflatten=columns")]
    pub columns: u64,

    #[serde(rename = "legend")]
    pub legend: XportLegend,
}

#[derive(Serialize)]
#[serde(rename = "v", transparent)]
pub struct XportValue {
    pub value: f64,
}

#[derive(Serialize)]
#[serde(rename = "row")]
pub struct XportRow {
    #[serde(rename = "$unflatten=t")]
    pub timestamp: u64,

    #[serde(rename = "$value")]
    pub values: Vec<XportValue>,
}

#[derive(Serialize)]
#[serde(rename = "xport")]
pub struct RrdXport {
    pub meta: XportMetadata,
    #[serde(rename = "$unflatten=data")]
    pub data: Vec<XportRow>,
    #[serde(rename = "$unflatten=script")]
    pub script: (),
}
