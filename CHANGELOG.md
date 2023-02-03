# Unreleased

* Add the leftwm widget to cnx-contrib
* Add ability to specify bar offset and width

# v0.3.1

* Add ability to define colors for widgets' attributes with RGB values or Hex color codes.
* Extend pager widget to mark hidden desktops that contain windows.
* Fix calculation of cpu usage.
* Add widget for commands. Shows the output of configured cli commands.

# v0.3.0

* Update to tokio 1.2.0 (from 0.2)
* Use tokio-stream package for streams.
* Add widget for cpu which shows cpu consumption (Code directly ported from xmobar)
* Add widget for wireless card. Shows your WiFi strength.
* Modify existing widgets to support custom template
* Use Pango's markup syntax to colorize and make it pretty
* Add widget for weather. Uses weathernoaa package to achieve the same.
