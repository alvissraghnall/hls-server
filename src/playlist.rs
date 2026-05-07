pub struct PlayList {}

enum PlayListType {
    MEDIA,
    MULTIVARIANT,
}

impl PlayList {
    // pub const TYPE: PlayListType;

    pub const fn new() -> Self {
        Self {}
    }
}
