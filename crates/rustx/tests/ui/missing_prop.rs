use rustx::prelude::*;
#[component]fn Card(title:String)->View{Node::text(title)}
fn main(){let _=rustx!{<Card/>};}
