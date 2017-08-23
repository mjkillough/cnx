// This will not be needed in error-chain 0.11:
#![allow(unused_doc_comment)]

error_chain!{
    foreign_links {
        Io(::std::io::Error);
        Alsa(::alsa::Error);
    }
}
