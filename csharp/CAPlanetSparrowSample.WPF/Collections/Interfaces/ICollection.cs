using System.Collections.Specialized;
using System.ComponentModel;

namespace System.Collections.ObjectModel;

public interface ICollection
{
    public event PropertyChangedEventHandler? PropertyChanged;
    public event NotifyCollectionChangedEventHandler? CollectionChanged;
    public event NotifyChildPropertyChangedEventHandler? ChildPropertyChanged;
}
