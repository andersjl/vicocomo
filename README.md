# Vicocomo - Slim MVC-DCI framework in Rust

W.I.P

This is a simple framework for the Model-View-Controller and
Data-Context-Interaction patterns, written in Rust.


## Concerns

The name is short for View, Controller, Context, Model.  The idea is that
separation of concerns.  Code for the Controller, Context, and Model concerns
should be kept in the `controllers`, `contexts`, and `models` directories.
Code for the View concern is split between the server and the browser.  Given
that you probably want to use a template engine on the server and Javascript
or Rust in the browser, the View software should be kept in the directories
`scripts`, `templates`, and `views`.

The figure hopefully clarifies how this relates to MVC and DCI.  Note that the
"View" here is a subset of the "V" of MVC.  In the following, "view" always
refers to this narrower concern.  As indicated, we recommend channeling *all*
user requests through a `Controller`, including requests for a specific *look*
rather than a specific *content*.

<br />
<img src="doc/conceptual-model.png">
<br />
<br />


## The framework software


## View

#### Browser Rust or Javascript code in the `src/script` directory


#### Template code in the `templates` directory


#### Server Rust code in the `view` directory

There is a trait `View` that contains some functions that may be useful
regardless of what the view is about.  Your views may use this trait.

The view may take parameters from the Controller to specify what should be
viewed and how it should look.

The trait implements a a tree structure for views ("include" in the figure
above).  A view that manages an HTML response and is the root of a view
include tree must prepare and render a template.

A view that is involved in an HTML response and is not the root of a view
include tree may prepare a template but not render it.  The templates should
have an include structure that mirrors the view tree.


## Controller


## Context

### General contexts



## Model
