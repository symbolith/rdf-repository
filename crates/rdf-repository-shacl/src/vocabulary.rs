use oxrdf::NamedNodeRef;

pub mod md {
    use super::*;

    pub const NAMESPACE: &str = "https://rdf-md.org/ns#";

    pub const DOCUMENT: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("https://rdf-md.org/ns#Document");

    pub const BODY: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("https://rdf-md.org/ns#body");
}

pub mod ptr {
    use super::*;

    pub const NAMESPACE: &str = "http://www.w3.org/2009/pointers#";

    pub const START_END_POINTER: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/2009/pointers#StartEndPointer");

    pub const LINE_CHAR_POINTER: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/2009/pointers#LineCharPointer");

    pub const SOURCE_POINTER: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/2009/pointers#sourcePointer");

    pub const START_POINTER: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/2009/pointers#startPointer");

    pub const END_POINTER: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/2009/pointers#endPointer");

    pub const LINE_NUMBER: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/2009/pointers#lineNumber");

    pub const CHAR_NUMBER: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/2009/pointers#charNumber");
}
