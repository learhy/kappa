use std::str::FromStr;

#[derive(Debug)]
pub enum Sample {
    Rate(u32),
    None,
}

impl FromStr for Sample {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        let base = split.next().map(u32::from_str);
        let rate = split.next().map(u32::from_str);

        let base = base.ok_or_else(|| format!("missing base: {}", s))?;
        let rate = rate.ok_or_else(|| format!("missing rate: {}", s))?;

        match (base, rate) {
            (Ok(1), Ok(n)) => Ok(Sample::Rate(n)),
            (Ok(n), _    ) => Err(format!("invalid base: {}", n)),
            _              => Err(format!("invalid rate: {}", s)),
        }
    }
}
