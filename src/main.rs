use regex::Regex;
use std::{fs, path::PathBuf};

#[derive(Debug)]
struct ImageFile {
    url: String,
    name: String,
}

#[derive(Debug)]
struct HTMLFile {
    url: String,
    html_body: Option<String>,
    html_document_title: String,
}

fn main() {
    const BASE_URL: &str = "https://susy.ic.unicamp.br:9999/mc202ABC";

    let html = make_request(BASE_URL);
    let sub_pages_directories = get_element_by_regex(r"\.\./.\./mc202ABC/\d*", &html);

    // Append the sub-directives to the base url
    let sub_pages_urls: Vec<HTMLFile> = sub_pages_directories
        .into_iter()
        .map(|sub_url| HTMLFile {
            url: format!("{}/{}/enunc.html", BASE_URL, sub_url),
            html_document_title: sub_url.to_string().replace("../../mc202ABC/", ""),
            html_body: None,
        })
        .collect();

    // Now that we have constructed the necessary URLS, iterate over them and download the necessary files
    for target_file in sub_pages_urls {
        println!("{:#?}", target_file);
        download_page(target_file);
    }
}

fn make_request(url: &str) -> String {
    // Make the request and check if it was a positive result or an Error
    let request_response = reqwest::blocking::get(url).unwrap_or_else(|error| {
        panic!(
            "[Error] Problem while making request to url {} : \n  {:?}",
            url, error
        )
    });

    // Return the html body
    request_response
        .text()
        .unwrap_or_else(|error| panic!("[ERROR] While trying to parse response body: {:?}", error))
}

fn get_element_by_regex<'t>(regex_rule: &str, text: &'t str) -> Vec<&'t str> {
    let re = Regex::new(regex_rule).unwrap();
    re.find_iter(text).map(|find| find.as_str()).collect()
}

fn download_page(mut target_file: HTMLFile) {
    // Get the HTML from the target URL
    target_file.html_body = Some(make_request(&target_file.url));

    // Parse the image urls present in the document
    let images_urls =
        get_document_images(target_file.url.replace(r"/enunc.html", ""), &target_file);

    // Now we can download the images and convert html to markdown
    download_images(&images_urls);
    convert_and_save(target_file);
    println!("{:#?}", images_urls);
}

fn download_images(image_files: &Vec<ImageFile>) {
    for image_file in image_files {
        // First make the request for the image
        let content = reqwest::blocking::get(&image_file.url)
            .unwrap()
            .bytes()
            .unwrap();

        // Save the contents to the file
        fs::write(&image_file.name, content).unwrap();
    }
}

fn convert_and_save(html: HTMLFile) {
    // Instantiate a new Pandoc session and set the correct input and output formats
    let mut pandoc = pandoc::new();
    pandoc.set_input_format(pandoc::InputFormat::Html, Vec::new());
    pandoc.set_output_format(pandoc::OutputFormat::MarkdownGithub, Vec::new());

    // Now pipe the input and convert
    pandoc.set_input(pandoc::InputKind::Pipe(html.html_body.unwrap().into()));
    pandoc.set_output(pandoc::OutputKind::File(PathBuf::from(
        html.html_document_title,
    )));

    pandoc.execute().unwrap();
}

fn get_document_images(base_url: String, html: &HTMLFile) -> Vec<ImageFile> {
    // Parse to get all image urls from the page
    let images = get_element_by_regex("<img .*>", html.html_body.as_ref().unwrap().as_str());

    // Clean the image tags
    let clean_image_file_names: Vec<ImageFile> = images
        .into_iter()
        .filter_map(|entry| clean_image_url(entry))
        .map(|valid_file_name| ImageFile {
            url: format!("{}/{}", base_url, valid_file_name),
            name: valid_file_name.to_string(),
        })
        .collect();

    return clean_image_file_names;
}

fn clean_image_url(raw_url: &str) -> Option<&str> {
    let re = Regex::new(r#"".*[png|jpg|jpeg|gif]""#).unwrap();
    let file_name_raw = re.captures(raw_url);
    match file_name_raw {
        Some(name) => {
            let match_value = name.get(0).unwrap().as_str();
            return Some(match_value.trim_matches('"'));
        }
        None => None,
    }
}
