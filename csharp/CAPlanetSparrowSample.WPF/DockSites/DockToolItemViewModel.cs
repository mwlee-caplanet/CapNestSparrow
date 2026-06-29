using ActiproSoftware.Windows.Controls;
using ActiproSoftware.Windows.Controls.Docking;
using ReactiveUI;
using ReactiveUI.SourceGenerators;
using System;
using System.Windows;

namespace CAPlanetSparrowSample.WPF.DockSites;

/// <summary>
/// Represents a tool item view-model.
/// </summary>
public partial class DockToolItemViewModel : DockItemViewModel
{
    /// <summary>
    /// Gets whether the container generated for this view model should be a tool window.
    /// </summary>
    /// <value>
    /// <c>true</c> if the container generated for this view model should be a tool window; otherwise, <c>false</c>.
    /// </value>
    public override bool IsTool => true;

    /// <summary>
    /// Gets or sets the default side that the tool window will dock towards when no prior location is known.
    /// </summary>
    /// <value>The default side that the tool window will dock towards when no prior location is known.</value>
    [Reactive] public Side DefaultDockSide { get; set; }

    /// <summary>
    /// Gets or sets the current state of the view.
    /// </summary>
    /// <value>The current state of the view.</value>
    [Reactive] public DockingWindowState State { get; set; }
    [Reactive] public bool? CanAutoHide { get; set; }
    [Reactive] public bool? CanBecomeDocument { get; set; }
    [Reactive] public bool? HasOptionsButton { get; set; }
    [Reactive] public bool? HasTitleBar { get; set; }
    [Reactive] public Size ContainerDockedSize { get; set; }
    [Reactive] public object? Content { get; set; }


    public DockToolItemViewModel(object content)
    {
        Content = content;
        this.WhenAnyValue(x => x.IsOpen).Subscribe(x => OpenMemento = x);
    }

    public bool? OpenMemento { get; set; } = null;
    public void ModeChange()
    {
        if (IsOpen)
        {
            IsOpen = false;
            OpenMemento = true;
        }
        else if (OpenMemento == true)
            IsOpen = true;
    }
}
