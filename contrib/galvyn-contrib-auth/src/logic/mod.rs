#[cfg(feature = "oidc")]
pub mod oidc;

#[cfg(not(feature = "oidc"))]
pub mod oidc {
    pub type Client = ();
}
