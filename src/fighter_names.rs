pub fn fighter_name(name: &str) -> String {
    match name {
        "captain"        => String::from("Captain Falcon"),
        "dedede"         => String::from("King Dedede"),
        "diddy"          => String::from("Diddy Kong"),
        "donkey"         => String::from("Donkey Kong"),
        "falco"          => String::from("Falco"),
        "fox"            => String::from("Fox"),
        "gamewatch"      => String::from("Game & Watch"),
        "ganon"          => String::from("Ganondorf"),
        "gkoopa"         => String::from("Gigabowser"),
        "ike"            => String::from("Ike"),
        "kirby"          => String::from("Kirby"),
        "koopa"          => String::from("Bowser"),
        "link"           => String::from("Link"),
        "lucario"        => String::from("Lucario"),
        "lucas"          => String::from("Lucas"),
        "luigi"          => String::from("Luigi"),
        "mario"          => String::from("Mario"),
        "marth"          => String::from("Marth"),
        "metaknight"     => String::from("Metaknight"),
        "ness"           => String::from("Ness"),
        "peach"          => String::from("Peach"),
        "pikachu"        => String::from("Pikachu"),
        "pit"            => String::from("Pit"),
        "pokefushigisou" => String::from("Ivysaur"),
        "pokelizardon"   => String::from("Charizard"),
        "poketrainer"    => String::from("Pokemon Trainer"),
        "pokezenigame"   => String::from("Squirtle"),
        "popo"           => String::from("Ice Climbers"),
        "purin"          => String::from("Jigglypuff"),
        "robot"          => String::from("Robot"),
        "samus"          => String::from("Samus"),
        "sheik"          => String::from("Sheik"),
        "snake"          => String::from("Snake"),
        "sonic"          => String::from("Sonic"),
        "szerosuit"      => String::from("Zerosuit Samus"),
        "toonlink"       => String::from("Toonlink"),
        "wario"          => String::from("Wario"),
        "warioman"       => String::from("Warioman"),
        "yoshi"          => String::from("Yoshi"),
        "zakoball"       => String::from("Wireframe Ball"),
        "zakoboy"        => String::from("Wireframe Boy"),
        "zakochild"      => String::from("Wireframe Child"),
        "zakogirl"       => String::from("Wireframe Girl"),
        "zelda"          => String::from("Zelda"),
        _                => name.to_string(),
    }
}
