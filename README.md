# alacritty-theme-switcher
Change alacritty themes.

You need to store the themes in in ~/.config/alacritty/themes/

## install
```console
$ cargo install --path .
```

## run
```console
$ alacritty-theme-switcher
```

## use fzf
```
$ alacritty-theme-switcher $(ls ~/.config/alacritty/themes | fzf)
```
