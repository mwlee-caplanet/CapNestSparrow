namespace System.Collections.ObjectModel;
public class NotifyChildPropertyChangedEventArgs : EventArgs
{
    public object Parent { get; }
    public int Index { get; }
    public object? Obj { get; }
    public string? PropertyName { get; }
    public NotifyChildPropertyChangedEventArgs(object parent, int index, object? obj, string? propertyName)
    {
        Parent = parent;
        Index = index;
        Obj = obj;
        PropertyName = propertyName;
    }

}

public delegate void NotifyChildPropertyChangedEventHandler(object? sender, NotifyChildPropertyChangedEventArgs e);
