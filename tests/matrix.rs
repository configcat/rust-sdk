use configcat::{Client, User};
use std::fs;

const NULL_VAL: &str = "##null##";

enum Kind {
    Value,
    Variation,
}

#[tokio::test]
async fn rollout_v1() {
    run(
        "testmatrix.csv",
        "PKDVCLf-Hq-h-kCzMp-L7Q/psuH7BGHoUmdONrzzUOY7A",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn rollout_v2() {
    run(
        "testmatrix.csv",
        "configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/AG6C1ngVb0CvM07un6JisQ",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn segments_v1() {
    run(
        "testmatrix_segments_old.csv",
        "PKDVCLf-Hq-h-kCzMp-L7Q/LcYz135LE0qbcacz2mgXnA",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn segments_v2() {
    run(
        "testmatrix_segments_old.csv",
        "configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/y_ZB7o-Xb0Swxth-ZlMSeA",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn semver1_v1() {
    run(
        "testmatrix_semantic.csv",
        "PKDVCLf-Hq-h-kCzMp-L7Q/BAr3KgLTP0ObzKnBTo5nhA",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn semver1_v2() {
    run(
        "testmatrix_semantic.csv",
        "configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/iV8vH2MBakKxkFZylxHmTg",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn semver2_v1() {
    run(
        "testmatrix_semantic_2.csv",
        "PKDVCLf-Hq-h-kCzMp-L7Q/q6jMCFIp-EmuAfnmZhPY7w",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn semver2_v2() {
    run(
        "testmatrix_semantic_2.csv",
        "configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/U8nt3zEhDEO5S2ulubCopA",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn number_v1() {
    run(
        "testmatrix_number.csv",
        "PKDVCLf-Hq-h-kCzMp-L7Q/uGyK3q9_ckmdxRyI7vjwCw",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn number_v2() {
    run(
        "testmatrix_number.csv",
        "configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/FCWN-k1dV0iBf8QZrDgjdw",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn sensitive_v1() {
    run(
        "testmatrix_sensitive.csv",
        "PKDVCLf-Hq-h-kCzMp-L7Q/qX3TP2dTj06ZpCCT1h_SPA",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn sensitive_v2() {
    run(
        "testmatrix_sensitive.csv",
        "configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/-0YmVOUNgEGKkgRF-rU65g",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn variation_v1() {
    run(
        "testmatrix_variationid.csv",
        "PKDVCLf-Hq-h-kCzMp-L7Q/nQ5qkhRAUEa6beEyyrVLBA",
        Kind::Variation,
    )
    .await
}

#[tokio::test]
async fn variation_v2() {
    run(
        "testmatrix_variationid.csv",
        "configcat-sdk-1/PKDVCLf-Hq-h-kCzMp-L7Q/spQnkRTIPEWVivZkWM84lQ",
        Kind::Variation,
    )
    .await
}

#[tokio::test]
async fn and_or() {
    run(
        "testmatrix_and_or.csv",
        "configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/ByMO9yZNn02kXcm72lnY1A",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn comp_v6() {
    run(
        "testmatrix_comparators_v6.csv",
        "configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/OfQqcTjfFUGBwMKqtyEOrQ",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn prerequisite() {
    run(
        "testmatrix_prerequisite_flag.csv",
        "configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/JoGwdqJZQ0K2xDy7LnbyOg",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn segments() {
    run(
        "testmatrix_segments.csv",
        "configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/h99HYXWWNE2bH8eWyLAVMA",
        Kind::Value,
    )
    .await
}

#[tokio::test]
async fn unicode() {
    run(
        "testmatrix_unicode.csv",
        "configcat-sdk-1/JcPbCGl_1E-K9M-fJOyKyQ/Da6w8dBbmUeMUBhh0iEeQQ",
        Kind::Value,
    )
    .await
}

async fn run(file_name: &str, sdk_key: &str, kind: Kind) {
    let client = Client::new(sdk_key).unwrap();

    let lines: Vec<String> = fs::read_to_string(format!("tests/data/{file_name}"))
        .unwrap()
        .lines()
        .map(String::from)
        .collect();

    let header: Vec<&str> = lines[0].split(';').collect();
    let custom_key = header[3];
    let keys: Vec<&str> = header.iter().map(|k| k.trim()).skip(4).collect();

    for line in lines.iter().skip(1) {
        let test_obj: Vec<&str> = line.split(';').map(|p| p.trim()).collect();
        if test_obj.len() == 1 {
            continue;
        }

        let mut user: Option<User> = None;
        if test_obj[0] != NULL_VAL {
            let mut u = User::new(test_obj[0]);

            if !test_obj[1].is_empty() && test_obj[1] != NULL_VAL {
                u = u.email(test_obj[1]);
            }

            if !test_obj[2].is_empty() && test_obj[2] != NULL_VAL {
                u = u.country(test_obj[2]);
            }

            if !test_obj[3].is_empty() && test_obj[3] != NULL_VAL {
                u = u.custom(custom_key, test_obj[3]);
            }
            user = Some(u);
        }

        for (ind, key) in keys.iter().enumerate() {
            let details = client.get_flag_details(key, user.clone()).await;
            let expected = test_obj[ind + 4];
            match kind {
                Kind::Value => {
                    let flag_val = details.value.unwrap();
                    let mut exp = expected.to_owned();
                    if flag_val.as_bool().is_some() {
                        exp = exp.to_lowercase();
                    }
                    assert_eq!(exp, format!("{flag_val}"))
                }
                Kind::Variation => {
                    assert_eq!(expected, details.variation_id.unwrap());
                }
            }
        }
    }
}
