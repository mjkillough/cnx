//! The error types used through out this crate.
//!
//! These are generated using [`error-chain`], and we recommend you also use
//! [`error-chain`] in any binary project using Cnx. This will allow you simple
//! error handling with rich error messages when things go wrong.
//!
//! See the included [`src/bin/cnx.rs`] for an example of how you might
//! integrate [`error-chain`] into your binary project along-side Cnx.
//!
//! [`error-chain`]: https://docs.rs/error-chain
//! [`src/bin/cnx.rs`]: https://github.com/mjkillough/cnx/blob/master/src/bin/cnx.rs

use error_chain::*;

error_chain!{
    foreign_links {
        Io(::std::io::Error);
        Alsa(::alsa::Error) #[cfg(feature = "volume-widget")];
    }
}
