use crate::parser::{
    jsx::{
        form_jsx_component_first_line, form_jsx_component_opening_line, parse_jsx_component,
        parse_jsx_component_first_line, JSXTagType,
    },
    HTMLTagType,
};
use nom::{
    error::{Error, ErrorKind},
    Err,
};

#[test]
pub fn test_form_jsx_component_first_line() {
    let mdx_line = "<Component />";
    assert_eq!(
        form_jsx_component_first_line(mdx_line, "Component"),
        Ok((
            "",
            (
                String::from("<Component />"),
                "",
                HTMLTagType::SelfClosing,
                0
            )
        ))
    );

    // let mdx_line = "<Component prop=\"prop/value\" />";
    // assert_eq!(
    //     form_jsx_component_first_line(mdx_line, "Component"),
    //     Ok((
    //         "",
    //         (
    //             String::from("<Component prop=\"prop/value\" />"),
    //             "prop=\"prop/value\" ",
    //             HTMLTagType::SelfClosing,
    //             0
    //         )
    //     ))
    // );

    let mdx_line = "<ComponentPure />";
    assert_eq!(
        form_jsx_component_first_line(mdx_line, "Component"),
        Err(Err::Error(Error::new("Pure", ErrorKind::Eof)))
    );

    let mdx_line = "<Component";
    assert_eq!(
        form_jsx_component_first_line(mdx_line, "Component"),
        Ok((
            "",
            (String::from("<Component"), "", HTMLTagType::OpeningStart, 0)
        ))
    );
}

#[test]
pub fn test_parse_jsx_component() {
    let mdx_line = "<Questions {questions} />";
    assert_eq!(
        parse_jsx_component(mdx_line, "Questions"),
        Ok(("", " {questions} "))
    );
}

#[test]
pub fn test_parse_jsx_component_first_line() {
    let mdx_line = "<CodeFragment";
    assert_eq!(
        parse_jsx_component_first_line(mdx_line, "CodeFragment"),
        Ok(("", ("<CodeFragment", &JSXTagType::Opened)))
    );

    let mdx_line = "<CodeFragment count={3} >";
    assert_eq!(
        parse_jsx_component_first_line(mdx_line, "CodeFragment"),
        Ok(("", ("<CodeFragment count={3} >", &JSXTagType::Closed)))
    );

    let mdx_line = "<CodeFragment count={3} />";
    assert_eq!(
        parse_jsx_component_first_line(mdx_line, "CodeFragment"),
        Ok(("", ("<CodeFragment count={3} />", &JSXTagType::SelfClosed)))
    );
}

#[test]
pub fn test_form_jsx_component_opening_line() {
    let mdx_line = "name=\"some name\"";
    assert_eq!(
        form_jsx_component_opening_line(mdx_line),
        Ok((
            "",
            (
                String::from("name=\"some name\""),
                "name=\"some name\"",
                HTMLTagType::OpeningStart,
                0
            )
        ))
    );
}
