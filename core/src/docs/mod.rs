//! Generazione documentazione a partire dal modello di progetto.
//!
//! Per l'MVP e' disponibile l'output Markdown ([`markdown`]). HTML, PDF ed
//! export verso Wiki (Confluence/Azure DevOps) sono previsti in roadmap e
//! partiranno tutti dallo stesso modello.

pub mod html;
pub mod markdown;
pub mod pdf;
pub mod wiki;
