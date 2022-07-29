/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use murin::*;
use zeroize::Zeroize;

fn main() -> Result<(),MurinError> {
    let plaintext = rpassword::prompt_password_stdout("cipher:")?;
    let mut password = rpassword::prompt_password_stdout("password:").unwrap();
    let cipher = hugin::encryption::encrypt(&plaintext,&password).unwrap();
    println!("Plaintext:\n {}\n",plaintext);
    println!("Ciphertext:\n {}\n",cipher);
    password.zeroize();
   

    println!("Decrypt: \n");
    let cipher = rpassword::prompt_password_stdout("ciphertext:")?;
    let mut password = rpassword::prompt_password_stdout("password:")?;
    let wallet_decr = hugin::encryption::decrypt(&cipher,&password).expect("Could not encrypt data");
    password.zeroize();
    println!("Decrypted: \n{}\n",wallet_decr);

    Ok(())
}