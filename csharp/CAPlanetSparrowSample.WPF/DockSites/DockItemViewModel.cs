using ReactiveUI;
using System;
using System.Collections.Generic;
using System.Text;
using System.Windows.Media;

using ReactiveUI.SourceGenerators;

namespace CAPlanetSparrowSample.WPF.DockSites;

/// <summary>
/// Represents a base class for all docking item view-models.
/// </summary>
public abstract class DockItemViewModel : ReactiveObject
{
    /// <summary>
    /// Gets or sets the description associated with the view-model.
    /// </summary>
    /// <value>The description associated with the view-model.</value>
    [Reactive] public string? Description { get; set; }

    /// <summary>
    /// Gets or sets the image associated with the view-model.
    /// </summary>
    /// <value>The image associated with the view-model.</value>
    [Reactive] public ImageSource? ImageSource { get; set; }

    /// <summary>
    /// Gets or sets whether the view is currently active.
    /// </summary>
    /// <value>
    /// <c>true</c> if the view is currently active; otherwise, <c>false</c>.
    /// </value>
    [Reactive] public bool IsActive { get; set; }

    /// <summary>
    /// Gets or sets whether the view is floating.
    /// </summary>
    /// <value>
    /// <c>true</c> if the view is floating; otherwise, <c>false</c>.
    /// </value>
    [Reactive] public bool IsFloating { get; set; }

    /// <summary>
    /// Gets or sets whether the view is currently open.
    /// </summary>
    /// <value>
    /// <c>true</c> if the view is currently open; otherwise, <c>false</c>.
    /// </value>
    [Reactive] public bool IsOpen { get; set; }

    /// <summary>
    /// Gets or sets whether the view is currently selected in its parent container.
    /// </summary>
    /// <value>
    /// <c>true</c> if the view is currently selected in its parent container; otherwise, <c>false</c>.
    /// </value>
    [Reactive] public bool IsSelected { get; set; }

    [Reactive] public string? SerializationId { get; set; }

    /// <summary>
    /// Gets or sets the title associated with the view-model.
    /// </summary>
    /// <value>The title associated with the view-model.</value>
    [Reactive] public string Title { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the window group name associated with the view-model.
    /// </summary>
    /// <value>The window group name associated with the view-model.</value>
    [Reactive] public string? WindowGroupName { get; set; }

    /// <summary>
    /// Gets whether the container generated for this view model should be a tool window.
    /// </summary>
    /// <value>
    /// <c>true</c> if the container generated for this view model should be a tool window; otherwise, <c>false</c>.
    /// </value>
    public abstract bool IsTool { get; }

    [Reactive] public bool? CanClose { get; set; }
    [Reactive] public bool? CanDock { get; set; }
    [Reactive] public bool? CanDragTab { get; set; }
    [Reactive] public bool? CanFloat { get; set; }
}
