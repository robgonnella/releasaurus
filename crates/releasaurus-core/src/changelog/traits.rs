use color_eyre::eyre::Result;

pub trait Generator {
    fn generate(&self) -> Result<String>;
}

pub trait CurrentVersion {
    fn current_version(&self) -> Option<String>;
}

pub trait NextVersion {
    fn next_version(&self) -> Option<String>;
    fn next_is_breaking(&self) -> Result<bool>;
}
