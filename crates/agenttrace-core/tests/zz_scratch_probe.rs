use agenttrace_core::{lookup_price, provider_for};

#[test]
fn probe_glm_winner() {
    for m in ["glm-5.2", "openrouter/claude-sonnet-4-20250514"] {
        let p = lookup_price(m);
        let prov = provider_for(m);
        println!("PROBE model={m:?} input={:?} output={:?} cw={:?} cr={:?} provider={:?}", p.input, p.output, p.cw, p.cr, prov);
    }
}
