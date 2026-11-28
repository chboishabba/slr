//! Candidate-only historical Wikifunctions acquisition. No arbitrary code
//! execution or inference that a historical lookup equals runtime selection.
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use std::io::Read;
use super::ProviderError;
const LIMIT:u64=8<<20;
fn invalid(s:&str)->ProviderError{ProviderError::InvalidInput(s.into())}
fn zid(s:&str)->bool{s.strip_prefix('Z').is_some_and(|v|!v.is_empty()&&!v.starts_with('0')&&v.bytes().all(|b|b.is_ascii_digit()))}
fn timestamp(s:&str)->bool{
    s.len()==20&&s.bytes().enumerate().all(|(i,b)|match i{
        4|7=>b==b'-',10=>b==b'T',13|16=>b==b':',19=>b==b'Z',_=>b.is_ascii_digit()
    })
}
fn request(url:&str)->Result<Vec<u8>,ProviderError>{
    let response=ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(25)).build()
        .get(url).set("Accept","application/json").set("User-Agent","SLR/0.1 historical Wikifunctions candidate acquisition").call()?;
    let mut bytes=Vec::new();response.into_reader().take(LIMIT+1).read_to_end(&mut bytes)?;
    if bytes.len() as u64>LIMIT{return Err(invalid("oversized revision response"));}
    Ok(bytes)
}
fn page_revision<'a>(v:&'a Value,id:&str)->Result<&'a Value,ProviderError>{
    let p=v["query"]["pages"].as_array().filter(|p|p.len()==1).ok_or_else(||invalid("missing or ambiguous page"))?;
    if p[0]["title"]!=id{return Err(invalid("foreign page"));}
    let r=p[0]["revisions"].as_array().filter(|r|r.len()==1).ok_or_else(||invalid("missing or ambiguous revision"))?;
    Ok(&r[0])
}
fn reference(v:&Value)->Option<&str>{
    v.as_str().or_else(||if v["Z1K1"]=="Z9"{v["Z9K1"].as_str()}else{None})
}
#[derive(Debug,Clone)]
pub struct HistoricalZObject{
    pub object_id:String,pub revision:u64,pub timestamp:String,pub sha256:String,pub bytes:Vec<u8>,
    pub receipt:Value,
}
pub fn parse_historical_zobject(id:&str,as_of:&str,revision:u64,bytes:&[u8])->Result<HistoricalZObject,ProviderError>{
    if !zid(id)||!timestamp(as_of)||revision==0||bytes.len() as u64>LIMIT{return Err(invalid("invalid acquisition coordinates"));}
    let value:Value=serde_json::from_slice(bytes).map_err(|_|invalid("invalid API JSON"))?;
    let r=page_revision(&value,id)?;
    let time=r["timestamp"].as_str().filter(|t|timestamp(t)&&*t<=as_of).ok_or_else(||invalid("invalid historical revision timestamp"))?;
    if r["revid"].as_u64()!=Some(revision){return Err(invalid("foreign revision"));}
    let slot=&r["slots"]["main"];
    if slot["contentmodel"]!="zobject"{return Err(invalid("unexpected content model"));}
    let raw=slot["content"].as_str().ok_or_else(||invalid("missing original content"))?.as_bytes().to_vec();
    let obj:Value=serde_json::from_slice(&raw).map_err(|_|invalid("invalid original Z object"))?;
    if reference(&obj["Z1K1"])!=Some("Z2")||reference(&obj["Z2K1"])!=Some(id){return Err(invalid("foreign persistent object"));}
    let sha256=format!("{:x}",Sha256::digest(&raw));
    let receipt=json!({"schema":"slr.historical-zobject-acquisition.v1","function":id,"revision":revision,
        "timestamp":time,"as_of":as_of,"sha256":sha256,"byte_length":raw.len(),
        "candidate_only":true,"runtime_dependency_resolution_paid":false,"world_truth_paid":false});
    Ok(HistoricalZObject{object_id:id.into(),revision,timestamp:time.into(),sha256,bytes:raw,receipt})
}
pub fn fetch_historical_zobject(id:&str,as_of:&str)->Result<HistoricalZObject,ProviderError>{
    if !zid(id)||!timestamp(as_of){return Err(invalid("invalid historical lookup coordinates"));}
    let base="https://www.wikifunctions.org/w/api.php?action=query&format=json&formatversion=2&prop=revisions";
    let lookup=format!("{base}&titles={id}&rvstart={as_of}&rvdir=older&rvlimit=1&rvprop=ids%7Ctimestamp");
    let v:Value=serde_json::from_slice(&request(&lookup)?).map_err(|_|invalid("invalid revision lookup"))?;
    let row=page_revision(&v,id)?;
    let revision=row["revid"].as_u64().filter(|r|*r>0).ok_or_else(||invalid("missing revision id"))?;
    let observed=row["timestamp"].as_str().filter(|s|timestamp(s)&&*s<=as_of).ok_or_else(||invalid("future or invalid lookup timestamp"))?;
    let exact=format!("{base}&revids={revision}&rvprop=ids%7Ctimestamp%7Ccontent&rvslots=main");
    let mut acquired=parse_historical_zobject(id,as_of,revision,&request(&exact)?)?;
    if acquired.timestamp!=observed{return Err(invalid("lookup and exact revision timestamps differ"));}
    acquired.receipt["lookup_url"]=json!(lookup);acquired.receipt["exact_revision_url"]=json!(exact);
    Ok(acquired)
}
#[cfg(test)]
mod tests{
    use super::*;
    fn fixture()->Value{json!({"query":{"pages":[{"title":"Z38757","revisions":[{"revid":1,"timestamp":"2026-10-15T23:00:00Z","slots":{"main":{"contentmodel":"zobject","content":"{\"Z1K1\":\"Z2\",\"Z2K1\":\"Z38757\",\"Z2K2\":{}}"}}}]}]}})}
    #[test]fn exact_revision_and_as_of(){
        let f=fixture();let b=serde_json::to_vec(&f).unwrap();
        assert!(parse_historical_zobject("Z38757","2026-10-15T23:37:24Z",1,&b).is_ok());
        assert!(parse_historical_zobject("Z38757","2026-10-15T22:37:24Z",1,&b).is_err());
        assert!(parse_historical_zobject("Z38757","2026-10-15T23:37:24Z",2,&b).is_err());
        assert!(parse_historical_zobject("Z35370","2026-10-15T23:37:24Z",1,&b).is_err());
    }
}
