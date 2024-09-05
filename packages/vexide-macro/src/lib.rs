//! This crate provides a procedural macro for marking the entrypoint of a [vexide](https://vexide.dev) program.

use parse::{Attrs, MacroOpts};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Signature};

mod parse;

const NO_SYNC_ERR: &str = "The vexide entrypoint must be marked `async`.";
const NO_UNSAFE_ERR: &str = "The vexide entrypoint must be not marked `unsafe`.";
const WRONG_ARGS_ERR: &str = "The vexide entrypoint must take a single parameter of type `vexide_devices::peripherals::Peripherals`";

fn verify_function_sig(sig: &Signature) -> Result<(), syn::Error> {
    let mut error = None;

    if sig.asyncness.is_none() {
        let message = syn::Error::new_spanned(sig, NO_SYNC_ERR);
        error.replace(message);
    }
    if sig.unsafety.is_some() {
        let message = syn::Error::new_spanned(sig, NO_UNSAFE_ERR);
        match error {
            Some(ref mut e) => e.combine(message),
            None => {
                error.replace(message);
            }
        };
    }
    if sig.inputs.len() != 1 {
        let message = syn::Error::new_spanned(sig, WRONG_ARGS_ERR);
        match error {
            Some(ref mut e) => e.combine(message),
            None => {
                error.replace(message);
            }
        };
    }

    match error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Marks a function as the entrypoint for a vexide program. When the program is started,
/// the `main` function will be called with a single argument of type `Peripherals` which
/// allows access to device peripherals like motors, sensors, and the display.
///
/// The `main` function must be marked `async` and must not be marked `unsafe`. It may
/// return any type that implements `Termination`, which includes `()`, `!`, and `Result`.
///
/// # Parameters
///
/// The `main` attribute can be provided with parameters that alter the behavior of the program.
///
/// - `banner`: A boolean value that toggles the vexide startup banner printed over serial.
///   When `false`, the banner will be not displayed.
///
/// # Examples
///
/// The most basic usage of the `main` attribute is to mark an async function as the entrypoint
/// for a vexide program. The function must take a single argument of type `Peripherals`.
///
/// ```ignore
/// # #![no_std]
/// # #![no_main]
/// # use vexide::prelude::*;
/// # use core::fmt::Write;
/// #[vexide::main]
/// async fn main(mut peripherals: Peripherals) {
///     write!(peripherals.screen, "Hello, vexide!").unwrap();
/// }
/// ```
///
/// The `main` attribute can also be provided with parameters to customize the behavior of the program.
///
/// ```ignore
/// # #![no_std]
/// # #![no_main]
/// # use vexide::prelude::*;
/// #[vexide::main(banner = false)]
/// async fn main(_p: Peripherals) {
///    println!("This is the only serial output from this program!")
/// }
/// ```
#[proc_macro_attribute]
pub fn main(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let inner = parse_macro_input!(item as ItemFn);
    let attrs = parse_macro_input!(attrs as Attrs);
    let opts = MacroOpts::from(attrs);

    match verify_function_sig(&inner.sig) {
        Ok(_) => {}
        Err(e) => return e.to_compile_error().into(),
    }
    let inner_ident = inner.sig.ident.clone();
    let ret_type = match &inner.sig.output {
        syn::ReturnType::Default => quote! { () },
        syn::ReturnType::Type(_, ty) => quote! { #ty },
    };
    let banner_print = opts.banner.then(|| {
        quote! { ::vexide::banner::print(); }
    });

    quote! {
        fn main() -> #ret_type {
            #inner

            #banner_print

            ::vexide::block_on(
                #inner_ident(::vexide::devices::peripherals::Peripherals::take().unwrap())
            )
        }
    }
    .into()
}

#[cfg(test)]
mod test {
    use syn::Ident;

    use super::*;

    #[test]
    fn wraps_main_fn() {
        let source = quote! {
            async fn main(_peripherals: Peripherals) {
                println!("Hello, world!");
            }
        };
        let input = syn::parse2::<ItemFn>(source).unwrap();
        let output = create_main_wrapper(input);
        assert_eq!(
            output.to_string(),
            quote! {
                fn main() {
                    async fn main(_peripherals: Peripherals) {
                        println!("Hello, world!");
                    }
                    let termination: () = ::vexide::async_runtime::block_on(
                        main(::vexide::devices::peripherals::Peripherals::take().unwrap())
                    );
                    ::vexide::core::program::Termination::report(termination);
                }
            }
            .to_string()
        );
    }

    #[test]
    fn toggles_banner_using_parsed_opts() {
        let entrypoint = make_entrypoint(MacroOpts { banner: false });
        assert!(entrypoint.to_string().contains("false"));
        assert!(!entrypoint.to_string().contains("true"));
        let entrypoint = make_entrypoint(MacroOpts { banner: true });
        assert!(entrypoint.to_string().contains("true"));
        assert!(!entrypoint.to_string().contains("false"));
    }

    #[test]
    fn requires_async() {
        let source = quote! {
            fn main(_peripherals: Peripherals) {
                println!("Hello, world!");
            }
        };
        let input = syn::parse2::<ItemFn>(source).unwrap();
        let output = create_main_wrapper(input);
        assert!(output.to_string().contains(NO_SYNC_ERR));
    }

    #[test]
    fn requires_safe() {
        let source = quote! {
            async unsafe fn main(_peripherals: Peripherals) {
                println!("Hello, world!");
            }
        };
        let input = syn::parse2::<ItemFn>(source).unwrap();
        let output = create_main_wrapper(input);
        assert!(output.to_string().contains(NO_UNSAFE_ERR));
    }

    #[test]
    fn disallows_0_args() {
        let source = quote! {
            async fn main() {
                println!("Hello, world!");
            }
        };
        let input = syn::parse2::<ItemFn>(source).unwrap();
        let output = create_main_wrapper(input);
        assert!(output.to_string().contains(WRONG_ARGS_ERR));
    }

    #[test]
    fn disallows_2_args() {
        let source = quote! {
            async fn main(_peripherals: Peripherals, _other: Peripherals) {
                println!("Hello, world!");
            }
        };
        let input = syn::parse2::<ItemFn>(source).unwrap();
        let output = create_main_wrapper(input);
        assert!(output.to_string().contains(WRONG_ARGS_ERR));
    }
}
