**Extracted StringBuilder append logic**
**Learning:** Repetitive boiler plate logic can easily clutter larger native files like `java_lang.rs`. Extracting shared behavior into a helper improves readability.
**Action:** Always look for repetitively duplicated boilerplate (like appending to an object's inner String) and extract it to a helper function.
