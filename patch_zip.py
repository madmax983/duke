import re

with open("crates/duke-loader/src/zip.rs", "r") as f:
    content = f.read()

# Replace find_resources
content = content.replace(
    '''    fn find_resources(&self, name: &str) -> Result<Vec<Vec<u8>>> {
        let mut resources = Vec::new();
        match self.reader.read_entry(name) {
            Ok(bytes) => resources.push(bytes),
            Err(Error::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }

        let mut boot_inf_name = format!("BOOT-INF/classes/{name}");
        match self.reader.read_entry(&boot_inf_name) {
            Ok(bytes) => resources.push(bytes),
            Err(Error::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }''',
    '''    fn find_resources(&self, name: &str) -> Result<Vec<Vec<u8>>> {
        let mut resources = Vec::new();
        match self.reader.read_entry(name) {
            Ok(bytes) => resources.push(bytes),
            Err(Error::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }

        // ⚡ Bolt: Eliminate intermediate String allocation and format! macro overhead
        let mut boot_inf_name = String::with_capacity(name.len() + 17);
        Self::append_boot_inf_classes_path(&mut boot_inf_name, name);
        match self.reader.read_entry(&boot_inf_name) {
            Ok(bytes) => resources.push(bytes),
            Err(Error::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }'''
)


# Replace find_resource_entries
content = content.replace(
    '''    fn find_resource_entries(&self, name: &str) -> Result<Vec<LocatedResource>> {
        let mut resources = Vec::new();
        match self.reader.read_entry(name) {
            Ok(bytes) => resources.push(LocatedResource {
                bytes,
                url: self.resource_url(name),
            }),
            Err(Error::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }

        let boot_inf_name = format!("BOOT-INF/classes/{name}");
        match self.reader.read_entry(&boot_inf_name) {
            Ok(bytes) => resources.push(LocatedResource {
                bytes,
                url: self.resource_url(&boot_inf_name),
            }),
            Err(Error::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }''',
    '''    fn find_resource_entries(&self, name: &str) -> Result<Vec<LocatedResource>> {
        let mut resources = Vec::new();
        match self.reader.read_entry(name) {
            Ok(bytes) => resources.push(LocatedResource {
                bytes,
                url: self.resource_url(name),
            }),
            Err(Error::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }

        // ⚡ Bolt: Eliminate intermediate String allocation and format! macro overhead
        let mut boot_inf_name = String::with_capacity(name.len() + 17);
        Self::append_boot_inf_classes_path(&mut boot_inf_name, name);
        match self.reader.read_entry(&boot_inf_name) {
            Ok(bytes) => resources.push(LocatedResource {
                bytes,
                url: self.resource_url(&boot_inf_name),
            }),
            Err(Error::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }'''
)

with open("crates/duke-loader/src/zip.rs", "w") as f:
    f.write(content)
