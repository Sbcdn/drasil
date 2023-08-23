use drasil_murin::*;
use zeroize::Zeroize;

fn main() -> Result<(), MurinError> {
    let plaintext = rpassword::prompt_password_stdout("cipher:")?;
    let mut password = rpassword::prompt_password_stdout("password:").unwrap();
    let cipher = drasil_hugin::encryption::encrypt(&plaintext, &password).unwrap();
    println!("Plaintext:\n {plaintext}\n");
    println!("Ciphertext:\n {cipher}\n");
    password.zeroize();

    println!("Decrypt: \n");
    let cipher = rpassword::prompt_password_stdout("ciphertext:")?;
    let mut password = rpassword::prompt_password_stdout("password:")?;
    let wallet_decr =
        drasil_hugin::encryption::decrypt(&cipher, &password).expect("Could not encrypt data");
    password.zeroize();
    println!("Decrypted: \n{wallet_decr}\n");

    Ok(())
}
