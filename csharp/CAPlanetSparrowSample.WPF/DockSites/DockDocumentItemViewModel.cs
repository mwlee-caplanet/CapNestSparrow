using System;
using System.Collections.Generic;
using System.Text;
using ReactiveUI.SourceGenerators;

namespace CAPlanetSparrowSample.WPF.DockSites;

/// <summary>
/// Represents a document item view-model.
/// </summary>
public class DockDocumentItemViewModel : DockItemViewModel
{
    /// <summary>
    /// Gets whether the container generated for this view model should be a tool window.
    /// </summary>
    /// <value>
    /// <c>true</c> if the container generated for this view model should be a tool window; otherwise, <c>false</c>.
    /// </value>
    public override bool IsTool => false;

    /// <summary>
    /// Gets or sets the file name associated with the view-model.
    /// </summary>
    /// <value>The file name associated with the view-model.</value>
    [Reactive] public string? FileName { get; set; }

    /// <summary>
    /// Gets or sets the read-only state of the associated with the view-model.
    /// </summary>
    /// <value>The read-only state of the associated with the view-model.</value>
    [Reactive] public bool IsReadOnly { get; set; }

    [Reactive] public object? Content { get; set; }

    public DockDocumentItemViewModel(object? content)
    {
        Content = content;
    }
}