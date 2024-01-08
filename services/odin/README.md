# Odin

## Summary
'Odin' is heart of the Drasil system: it builds and finalizes transactions as well as interacting with and composing the different parts.  'Odin' implements the blockchain applications and is the central runtime of the Drasil Application Framework. 
'Odin' itself is very lightwight, implementing the various blockchain interactions via 'Commands'. The communications protocol is 
created with the Redis. 

There is no direct documentation for 'Odin' as it essentially serves all existing functions. 'Odin' is an internal service and it is not possible to directly communicate with it. The gateway API to communicate with 'Odin' is 'Heimdallr'. This allows encapsulations between the central blockchain services and the available endpoints as well as independent scalability between the two services. 
By using a loadbalancer between several 'Heimdallr' and 'Odin' instances it is possible to scale the application depending on load. Each 'Odin' instance starts incoming application commands as asynchronous independent processes. 

* [Documentation](https://docs.drasil.io/)

## Mythology
Odin (/ˈoʊdɪn/; from Old Norse: Óðinn, IPA: [ˈoːðenː]) is a widely revered god in Germanic paganism. Norse mythology, the source of most surviving information about him, associates him with wisdom, healing, death, royalty, the gallows, knowledge, war, battle, victory, sorcery, poetry, frenzy, and the runic alphabet, and depicts him as the husband of the goddess Frigg. In wider Germanic mythology and paganism, the god was also known in Old English as Wōden, in Old Saxon as Uuôden, in Old Dutch as Wuodan, in Old Frisian as Wêda, and in Old High German as Wuotan, all ultimately stemming from the Proto-Germanic theonym *Wōđanaz, meaning 'lord of frenzy', or 'leader of the possessed'.

Odin appears as a prominent god throughout the recorded history of Northern Europe, from the Roman occupation of regions of Germania (from c.  2 BCE) through movement of peoples during the Migration Period (4th to 6th centuries CE) and the Viking Age (8th to 11th centuries CE). In the modern period the rural folklore of Germanic Europe continued to acknowledge Odin. References to him appear in place names throughout regions historically inhabited by the ancient Germanic peoples, and the day of the week Wednesday bears his name in many Germanic languages, including in English.
[Wikipedia](https://en.wikipedia.org/wiki/Odin)