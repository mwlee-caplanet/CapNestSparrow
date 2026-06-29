using CAPlanetSparrowSample.WPF.DockSites;
using ReactiveUI;
using ReactiveUI.SourceGenerators;
using System;
using System.Collections.Generic;
using System.Text;
using CAPlanetSparrowSample.WPF.Data;

namespace CAPlanetSparrowSample.WPF.Views;

public class WorkspaceViewModel : ReactiveObject
{
    public MainWindowViewModel ParentViewModel { get; }

    public DockDocumentItemViewModel DockingItem { get; }

    public WorkspaceViewModel(MainWindowViewModel ParentViewModel, SparrowInputData InputData)
    {
        this.ParentViewModel = ParentViewModel;
        DockingItem = new DockDocumentItemViewModel(this)
        {
            Title = InputData.ProjectName
        };
    }
}
