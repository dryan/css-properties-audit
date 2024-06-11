use lightningcss::declaration::DeclarationBlock;
use lightningcss::printer::{Printer, PrinterOptions};
use lightningcss::properties::custom::{TokenList, TokenOrValue};
use lightningcss::properties::Property;
use lightningcss::rules::keyframes::KeyframesName;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use serde::Serialize;
use std::env;
use xxhash_rust::xxh3::xxh3_64;

enum OutputFormats {
    Terminal,
    JSON,
    HTML,
    None,
}

const HTML_TEMPLATE: &str = r#"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>CSS Custom Properties Audit</title>
    <link
      rel="stylesheet"
      href="https://unpkg.com/@dryan-llc/mnml.css"
      crossorigin="anonymous"
    />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <style>
        html {
            scroll-behavior: smooth;
        }

        .audit-grid {
            display: grid;
            row-gap: 2rem;
            column-gap: 2rem;

            @container --mnml-container (width >= 48rem) {
                grid-template-columns: 1fr 3fr;            
            }
        }

        .site-header {
            grid-column: 1 / -1;
            display: grid;
            row-gap: 1rem;

            h1 {
                margin-block: 0;
            }
        }

        h2, css-audit-minimap {
            font-family: var(--mnml--font--monospace);
        }

        h2 ~ h2 {
            margin-block-start: 3rem;
        }

        h2 .count {
            font-family: var(--mnml--font--sans);
            font-size: 0.75em;
        }

        css-audit-minimap {
            display: flex;
            flex-direction: column;
            row-gap: 1rem;

            a.dimmed {
                pointer-events: none;
                opacity: 0.5;
            }
        }

        css-audit-search {
            display: flow-root;

            &:not(:defined) {
                display: none;
            }

            form {
                display: grid;
                grid-template-columns: 1fr auto;
                column-gap: 1rem;
            }
        }
    </style>
  </head>
  <body>
    <div class="container">
        <div class="audit-grid">
            <header class="site-header">
                <h1>CSS Custom Properties Audit</h1>

                <css-audit-search>
                    <form method="GET" action=".">
                        <label for="search" class="reader-only">Search</label>
                        <input type="search" id="search" name="search" />
                        <button type="submit" data-color="primary">Search</button>
                    </form>
                </css-audit-search>
            </header>

            <css-audit-minimap role="navigation"></css-audit-minimap>

            <main></main>

            <script type="module">
                class CssAuditMinimap extends HTMLElement {
                    connectedCallback() {}
                }

                customElements.define("css-audit-minimap", CssAuditMinimap);

                class CssAuditSearch extends HTMLElement {
                    connectedCallback() {
                        this.form = this.querySelector("form");
                        this.input = this.querySelector("input");
                        this.button = this.querySelector("button");

                        this.form.addEventListener("submit", this.handleSubmit.bind(this));
                    }

                    handleSubmit(event) {
                        event.preventDefault();
                        const search = this.input.value;
                        const headings = Array.from(document.querySelectorAll("h2"));
                        const matchingHeadings = headings.filter(heading => heading.textContent.includes(search));

                        if (matchingHeadings.length === 0) {
                            alert("No matches found");
                            return;
                        }

                        const matchingMinimapLinks = matchingHeadings.map(heading => {
                            const id = heading.getAttribute("id");
                            return document.querySelector(`a[href='#${id}']`);
                        });

                        const nonMatchingMinimapLinks = Array.from(document.querySelectorAll("css-audit-minimap a")).filter(link => !matchingMinimapLinks.includes(link));

                        matchingHeadings.forEach(heading => {
                            heading.removeAttribute('hidden');
                            heading.nextElementSibling.closest('ul')?.removeAttribute('hidden');
                        });

                        headings.filter(heading => !matchingHeadings.includes(heading)).forEach(heading => {
                            heading.setAttribute('hidden', true);
                            heading.nextElementSibling.closest('ul')?.setAttribute('hidden', true);
                        });

                        matchingMinimapLinks.forEach(link => link.classList.remove('dimmed'));
                        nonMatchingMinimapLinks.forEach(link => link.classList.add('dimmed'));
                    }
                }

                customElements.define("css-audit-search", CssAuditSearch);
            </script>
        </div>
    </div>
  </body>
</html>
"#;

// write a function that takes anything with a .to_css method and uses that to return a string
pub fn to_css(thing: impl ToCss) -> String {
    let mut dest = String::with_capacity(1);
    let mut printer = Printer::new(&mut dest, PrinterOptions::default());
    thing.to_css(&mut printer).unwrap();
    return dest;
}

#[derive(Serialize)]
struct CssRulesHashMap {
    selector: String,
    rules: Vec<String>,
}

fn handle_declarations(
    selectors: &Vec<String>,
    declarations: &DeclarationBlock,
) -> std::collections::HashMap<String, Vec<String>> {
    let mut custom_properties: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for declaration in &declarations.declarations {
        match declaration {
            Property::Unparsed(unparsed) => match &unparsed.value {
                TokenList(tokens) => {
                    for token in tokens {
                        match token {
                            TokenOrValue::Var(var) => {
                                // convert ident to string
                                let _ident = var.name.ident.to_string();
                                let ident = _ident.as_str();
                                // if ident starts with --__, skip it
                                if ident.starts_with("--__") {
                                    continue;
                                }
                                // if the ident isn't in custom_properties, add it as an empty array
                                if !custom_properties.contains_key(ident) {
                                    custom_properties.insert(ident.to_string(), vec![]);
                                }
                                for selector in selectors {
                                    // convert selector.iter() to a string as a variable
                                    custom_properties
                                        .get_mut(ident)
                                        .unwrap()
                                        .push(selector.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            },
            _ => {}
        }
    }

    return custom_properties;
}

fn main() {
    // if --help is passed as an argument, print the help message and exit

    if env::args().any(|x| x == "--help") {
        println!("Parses one or more CSS stylesheets and outputs a list of custom properties and the selectors that use them.");
        println!();
        println!("Usage: css-audit [options] <stylesheet>...");
        println!();
        println!("Options:");
        println!("  --help             Print this help message and exit");
        println!("  --format=terminal  Output to the terminal (default)");
        println!("  --format=html      Output an HTML document");
        println!("  --format=json      Output a JSON document");
        println!("  --format=none      Do not output anything (useful for testing)");
        println!();
        println!("Examples:");
        println!("  css-audit --format=html styles.css");
        println!("  css-audit --format=json styles.css");
        println!("  css-audit styles.css");
        std::process::exit(0);
    }

    let mut stylesheets: Vec<String> = env::args().collect();
    // remove the first argument, which is the program name
    stylesheets.remove(0);

    // get any --format argument and remove it from the stylesheets
    let format: OutputFormats = match stylesheets.iter().position(|x| x.starts_with("--format")) {
        Some(index) => {
            // support both format html and format=html
            let arg = stylesheets[index].clone();
            if arg.contains("=") {
                let format = arg.split("=").collect::<Vec<&str>>()[1];
                stylesheets.remove(index);
                match format {
                    "json" => OutputFormats::JSON,
                    "html" => OutputFormats::HTML,
                    "none" => OutputFormats::None,
                    _ => OutputFormats::Terminal,
                }
            } else {
                let format = stylesheets[index + 1].clone();
                stylesheets.remove(index + 1);
                stylesheets.remove(index);
                match format.as_str() {
                    "json" => OutputFormats::JSON,
                    "html" => OutputFormats::HTML,
                    "none" => OutputFormats::None,
                    _ => OutputFormats::Terminal,
                }
            }
        }
        None => OutputFormats::Terminal,
    };

    // remove any arguments that start with --
    stylesheets.retain(|x| !x.starts_with("--"));

    // if there are no stylesheets, print an error message and exit
    if stylesheets.len() == 0 {
        eprintln!("No stylesheets provided");
        std::process::exit(1);
    }

    // create a map of every custom property used in all stylesheets that contains
    // an array of selectors that use that property
    let mut custom_properties: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for path in &stylesheets {
        // get the contents of the stylesheet
        let contents = std::fs::read_to_string(path).expect("Failed to read stylesheet");
        let mut stylesheet = StyleSheet::parse(&contents, ParserOptions::default())
            .expect("Failed to parse stylesheet");
        // loop over every CSSRule in stylesheet.rules
        for mut rule in stylesheet.rules.0.drain(..) {
            match &mut rule {
                CssRule::Keyframes(rule) => {
                    // println!("Keyframes: {:?}", keyframes);
                    let name = match &rule.name {
                        KeyframesName::Ident(ident) => format!("@keyframes {}", ident.to_string()),
                        KeyframesName::Custom(string) => {
                            format!("@keyframes {}", string.to_string())
                        }
                    };
                    let selectors = vec![name.to_string()];
                    for keyframe in rule.keyframes.iter() {
                        let custom_properties_in_keyframe =
                            handle_declarations(&selectors, &keyframe.declarations);
                        for (key, value) in custom_properties_in_keyframe {
                            if !custom_properties.contains_key(&key) {
                                custom_properties.insert(key, value);
                            } else {
                                custom_properties.get_mut(&key).unwrap().extend(value);
                            }
                        }
                    }
                }
                CssRule::CustomMedia(media) => {
                    eprintln!("@custom-media is not supported: {:?}", media);
                }
                CssRule::Media(media) => {
                    let mq = format!("@media {}", to_css(&media.query));
                    for rule in &media.rules.0 {
                        match rule {
                            CssRule::Style(style) => {
                                let selectors = style.selectors.0.to_vec();
                                let selectors_as_strings: Vec<String> = selectors
                                    .iter()
                                    .map(|selector| format!("{}\n    {:?}", mq, selector.iter()))
                                    .collect();
                                let custom_properties_in_style =
                                    handle_declarations(&selectors_as_strings, &style.declarations);
                                for (key, value) in custom_properties_in_style {
                                    if !custom_properties.contains_key(&key) {
                                        custom_properties.insert(key, value);
                                    } else {
                                        custom_properties.get_mut(&key).unwrap().extend(value);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                CssRule::Supports(supports) => {
                    // println!("Supports: {:?}", supports);
                    let at_supports = format!("@supports {}", to_css(&supports.condition));
                    for rule in &supports.rules.0 {
                        match rule {
                            CssRule::Style(style) => {
                                let selectors = style.selectors.0.to_vec();
                                let selectors_as_strings: Vec<String> = selectors
                                    .iter()
                                    .map(|selector| {
                                        format!("{}\n    {:?}", at_supports, selector.iter())
                                    })
                                    .collect();
                                let custom_properties_in_style =
                                    handle_declarations(&selectors_as_strings, &style.declarations);
                                for (key, value) in custom_properties_in_style.into_iter() {
                                    if !custom_properties.contains_key(&key) {
                                        custom_properties.insert(key, value);
                                    } else {
                                        custom_properties.get_mut(&key).unwrap().extend(value);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                CssRule::Container(container) => {
                    // println!("Container: {:?}", container);
                    let at_container = format!("@container {}", to_css(&container.condition));
                    for rule in &container.rules.0 {
                        match rule {
                            CssRule::Style(style) => {
                                let selectors = style.selectors.0.to_vec();
                                let selectors_as_strings: Vec<String> = selectors
                                    .iter()
                                    .map(|selector| {
                                        format!("{}\n    {:?}", at_container, selector.iter())
                                    })
                                    .collect();
                                let custom_properties_in_style =
                                    handle_declarations(&selectors_as_strings, &style.declarations);
                                for (key, value) in custom_properties_in_style.into_iter() {
                                    if !custom_properties.contains_key(&key) {
                                        custom_properties.insert(key, value);
                                    } else {
                                        custom_properties.get_mut(&key).unwrap().extend(value);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                CssRule::LayerBlock(layer_block) => {
                    let at_layer: String = if Option::is_some(&layer_block.name) {
                        match &layer_block.name {
                            Some(name) => format!("@layer {}", to_css(name)),
                            None => "@layer".to_string(),
                        }
                    } else {
                        "@layer".to_string()
                    };
                    for rule in &layer_block.rules.0 {
                        match rule {
                            CssRule::Style(style) => {
                                let selectors = style.selectors.0.to_vec();
                                let selectors_as_strings: Vec<String> = selectors
                                    .iter()
                                    .map(|selector| {
                                        format!("{}\n    {:?}", at_layer, selector.iter())
                                    })
                                    .collect();
                                let custom_properties_in_style =
                                    handle_declarations(&selectors_as_strings, &style.declarations);
                                for (key, value) in custom_properties_in_style.into_iter() {
                                    if !custom_properties.contains_key(&key) {
                                        custom_properties.insert(key, value);
                                    } else {
                                        custom_properties.get_mut(&key).unwrap().extend(value);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                CssRule::Style(style) => {
                    let selectors = style.selectors.0.to_vec();
                    let selectors_as_strings: Vec<String> = selectors
                        .iter()
                        .map(|selector| format!("{:?}", selector.iter()))
                        .collect();
                    let custom_properties_in_style =
                        handle_declarations(&selectors_as_strings, &style.declarations);
                    for (key, value) in custom_properties_in_style {
                        if !custom_properties.contains_key(&key) {
                            custom_properties.insert(key, value);
                        } else {
                            custom_properties.get_mut(&key).unwrap().extend(value);
                        }
                    }
                }
                CssRule::Scope(scope) => {
                    // println!("Scope: {:?}", scope);
                    eprintln!("@scope is not supported: {:?}", scope);
                }
                CssRule::Nesting(nesting) => {
                    // println!("Nesting: {:?}", nesting);
                    eprintln!("nesting is not supported: {:?}", nesting);
                }
                CssRule::StartingStyle(starting_style) => {
                    // println!("StartingStyle: {:?}", starting_style);
                    eprintln!("@starting-style is not supported: {:?}", starting_style);
                }
                CssRule::Property(property) => {
                    // println!("Property: {:?}", property);
                    eprintln!("@property is not supported: {:?}", property);
                }
                _ => {}
            }
        }
    }

    // sort and dedup the selectors in each custom property
    for (_, selectors) in &mut custom_properties {
        selectors.sort();
        selectors.dedup();
    }

    // sort the custom properties by key
    let mut custom_properties: Vec<(&String, &Vec<String>)> = custom_properties.iter().collect();
    custom_properties.sort_by(|a, b| a.0.cmp(b.0));

    match format {
        OutputFormats::Terminal => {
            let mut loop_count = 0;
            for (key, value) in custom_properties {
                if loop_count > 0 {
                    println!();
                }
                println!("{}", key);
                for selector in value {
                    println!("  {}", selector);
                }
                loop_count += 1;
            }
        }
        OutputFormats::JSON => {
            // output JSON as a list of [{selector: string, rules: [string]}]
            let mut json: Vec<CssRulesHashMap> = vec![];
            for (key, value) in custom_properties {
                let map = CssRulesHashMap {
                    selector: key.to_string(),
                    rules: value.clone(),
                };
                json.push(map);
            }
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        OutputFormats::HTML => {
            // replace the contents of HTML_TEMPLATE's <main> with h2 and ul elements
            // then replace the contents of css-audit-minimap with links to the h2 elements
            // use a hashed version of the selector as the id for the h2 elements

            let template = HTML_TEMPLATE.to_string();
            let mut sections: Vec<String> = vec![];
            let mut minimap: Vec<String> = vec![];
            for (key, value) in custom_properties {
                let id: String = format!("selector-{:x}", xxh3_64(key.as_bytes()));
                let h2 = format!(
                    "<h2 id=\"{}\">{} <span class='count'>({})</span></h2>",
                    id,
                    key,
                    value.len()
                );
                let ul = format!(
                    "<ul>{}</ul>",
                    value
                        .into_iter()
                        .map(|v| format!("<li>{}</li>", v))
                        .collect::<Vec<String>>()
                        .join("")
                );
                sections.push(format!("{}{}", h2, ul));
                minimap.push(format!("<a href=\"#{}\">{}</a>", id, key));
            }
            let sections = sections.join("");
            let minimap = minimap.join("");
            let html = template
                .replace("</main>", format!("{}</main>", &sections)[..].as_ref())
                .replace(
                    "</css-audit-minimap>",
                    format!("{}</css-audit-minimap>", &minimap)[..].as_ref(),
                );
            println!("{}", html);
        }
        OutputFormats::None => {}
    }
}
